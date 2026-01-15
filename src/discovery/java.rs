//! Spring Boot source code parser (Java/Kotlin)

use std::path::Path;
use std::fs;
use std::sync::OnceLock;
use regex::Regex;
use crate::discovery::models::{
    AuthRequirement, DiscoveredEndpoint, WorkspaceProject, Framework,
};

/// Parse Spring Boot source files
pub fn parse_java_routes(project_root: &Path) -> Vec<DiscoveredEndpoint> {
    let mut endpoints = Vec::new();
    let java_files = find_java_files(project_root);
    
    for file_path in java_files {
        if let Ok(content) = fs::read_to_string(&file_path) {
            endpoints.extend(parse_spring_file(&content, &file_path));
        }
    }
    
    endpoints
}

fn find_java_files(root: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                if !path.ends_with("target") && !path.ends_with(".git") && !path.ends_with(".gradle") {
                    files.extend(find_java_files(&path));
                }
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext == "java" || ext == "kt" {
                    files.push(path);
                }
            }
        }
    }
    files
}

fn parse_spring_file(content: &str, file_path: &Path) -> Vec<DiscoveredEndpoint> {
    static CLASS_MAPPING: OnceLock<Regex> = OnceLock::new();
    static METHOD_MAPPING: OnceLock<Regex> = OnceLock::new();
    
    // @RequestMapping("/api/v1") on class
    let class_mapping_re = CLASS_MAPPING.get_or_init(|| {
        Regex::new(r#"@RequestMapping\s*\(\s*(?:value\s*=\s*)?["']([^"']+)["']"#).unwrap()
    });
    
    // @GetMapping("/users"), @PostMapping(value = "/path")
    let method_mapping_re = METHOD_MAPPING.get_or_init(|| {
        Regex::new(r#"@(GetMapping|PostMapping|PutMapping|DeleteMapping|PatchMapping|RequestMapping)\s*\(\s*(?:value\s*=\s*)?["']([^"']+)["']"#).unwrap()
    });

    let mut endpoints = Vec::new();
    let mut class_prefix = String::new();
    
    if let Some(caps) = class_mapping_re.captures(content) {
        if let Some(prefix) = caps.get(1) {
            class_prefix = prefix.as_str().trim_end_matches('/').to_string();
            if !class_prefix.starts_with('/') {
                class_prefix = format!("/{}", class_prefix);
            }
        }
    }
    
    for (line_num, line) in content.lines().enumerate() {
        if let Some(caps) = method_mapping_re.captures(line) {
            let annotation = caps.get(1).map(|m| m.as_str()).unwrap_or("GetMapping");
            
            // If it's @RequestMapping, check if it's applying to a class (lookahead)
            if annotation == "RequestMapping" {
                 let next_lines: String = content.lines()
                    .skip(line_num + 1)
                    .take(2)
                    .collect::<Vec<_>>()
                    .join("\n");
                if next_lines.contains("class ") || line.contains("class ") {
                    continue;
                }
            }

            let path_part = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            
            let method = match annotation {
                "GetMapping" => "GET",
                "PostMapping" => "POST",
                "PutMapping" => "PUT",
                "DeleteMapping" => "DELETE",
                "PatchMapping" => "PATCH",
                "RequestMapping" => "ANY",
                _ => "GET",
            };
            
            let mut full_path = format!("{}{}", class_prefix, path_part);
            if !full_path.starts_with('/') {
                full_path = format!("/{}", full_path);
            }
            if full_path.len() > 1 && full_path.ends_with('/') {
                full_path.pop();
            }
            
            let mut endpoint = DiscoveredEndpoint::new(method, full_path);
            endpoint.source_file = Some(file_path.to_path_buf());
            endpoint.line_number = Some(line_num + 1);
            endpoint.auth = AuthRequirement::None; // Hard to detect auth reliably without complex static analysis
            
            endpoints.push(endpoint);
        }
    }
    
    endpoints
}

/// Load a Spring Boot project
pub fn load_java_project(project_root: &Path) -> WorkspaceProject {
    let mut project = WorkspaceProject::new(project_root.to_path_buf());
    project.framework = Framework::SpringBoot;
    project.endpoints = parse_java_routes(project_root);
    // Base URL often 8080
    project.base_url = Some("http://localhost:8080".to_string());
    project
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_spring() {
        let content = r#"
@RestController
@RequestMapping("/api/v1")
public class UserController {

    @GetMapping("/users")
    public List<User> getUsers() {}

    @PostMapping(value = "/users")
    public User createUser() {}
}
"#;
        let endpoints = parse_spring_file(content, Path::new("UserController.java"));
        assert_eq!(endpoints.len(), 2);
        
        let get = endpoints.iter().find(|e| e.method == "GET").unwrap();
        assert_eq!(get.path, "/api/v1/users");
        
        let post = endpoints.iter().find(|e| e.method == "POST").unwrap();
        assert_eq!(post.path, "/api/v1/users");
    }
}
