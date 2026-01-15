//! NestJS source code parser (TypeScript)

use std::path::Path;
use std::fs;
use std::sync::OnceLock;
use regex::Regex;
use crate::discovery::models::{
    AuthRequirement, DiscoveredEndpoint, WorkspaceProject, Framework,
};

pub fn parse_nestjs_routes(project_root: &Path) -> Vec<DiscoveredEndpoint> {
    let mut endpoints = Vec::new();
    let ts_files = find_ts_files(project_root);
    
    for file_path in ts_files {
        if let Ok(content) = fs::read_to_string(&file_path) {
            endpoints.extend(parse_nestjs_file(&content, &file_path));
        }
    }
    
    endpoints
}

fn find_ts_files(root: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                if !path.ends_with("node_modules") && !path.ends_with("dist") && !path.ends_with(".git") {
                    files.extend(find_ts_files(&path));
                }
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext == "ts" {
                    files.push(path);
                }
            }
        }
    }
    files
}

fn parse_nestjs_file(content: &str, file_path: &Path) -> Vec<DiscoveredEndpoint> {
    static CONTROLLER_RE: OnceLock<Regex> = OnceLock::new();
    static METHOD_RE: OnceLock<Regex> = OnceLock::new();
    
    // @Controller('users')
    let controller_re = CONTROLLER_RE.get_or_init(|| {
        Regex::new(r#"@Controller\s*\(\s*['"]([^'"]*)['"]"#).unwrap()
    });
    
    // @Get(':id'), @Post('create'), @Post()
    let method_re = METHOD_RE.get_or_init(|| {
        Regex::new(r#"@(Get|Post|Put|Delete|Patch|All|Options|Head)\s*\(\s*(?:['"]([^'"]*)['"])?"#).unwrap()
    });

    let mut endpoints = Vec::new();
    let mut controller_path = String::new();
    
    if let Some(caps) = controller_re.captures(content) {
        if let Some(path) = caps.get(1) {
            controller_path = path.as_str().to_string();
            if !controller_path.is_empty() && !controller_path.starts_with('/') {
                controller_path = format!("/{}", controller_path);
            }
        }
    }
    
    for (line_num, line) in content.lines().enumerate() {
        if let Some(caps) = method_re.captures(line) {
            let method = caps.get(1).map(|m| m.as_str().to_uppercase()).unwrap_or("GET".to_string());
            let path_part = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            
            let mut full_path = format!("{}{}", controller_path, if !path_part.is_empty() && !path_part.starts_with('/') {
                format!("/{}", path_part)
            } else {
                path_part.to_string()
            });

            if full_path.is_empty() {
                full_path = "/".to_string();
            }
            
            let mut endpoint = DiscoveredEndpoint::new(method, full_path);
            endpoint.source_file = Some(file_path.to_path_buf());
            endpoint.line_number = Some(line_num + 1);
            
            if line.contains("@UseGuards") {
                endpoint.auth = AuthRequirement::Bearer; // Assume guards imply auth
            }
            
            endpoints.push(endpoint);
        }
    }
    
    endpoints
}

pub fn load_nestjs_project(project_root: &Path) -> WorkspaceProject {
    let mut project = WorkspaceProject::new(project_root.to_path_buf());
    project.framework = Framework::NestJS;
    project.endpoints = parse_nestjs_routes(project_root);
    project.base_url = Some("http://localhost:3000".to_string());
    project
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_nestjs() {
        let content = r#"
@Controller('cats')
export class CatsController {
  @Post()
  create(): string {
    return 'This action adds a new cat';
  }

  @Get()
  findAll(): string {
    return 'This action returns all cats';
  }
  
  @Get(':id')
  findOne(@Param('id') id: string) {
    return `This action returns a #${id} cat`;
  }
}
"#;
        let endpoints = parse_nestjs_file(content, Path::new("cats.controller.ts"));
        assert_eq!(endpoints.len(), 3);
        
        let post = endpoints.iter().find(|e| e.method == "POST").unwrap();
        assert_eq!(post.path, "/cats");
        
        let get = endpoints.iter().find(|e| e.method == "GET" && e.path == "/cats").unwrap();
        assert!(get !=  endpoints.iter().find(|e| e.method == "GET" && e.path == "/cats/:id").unwrap());
    }
}
