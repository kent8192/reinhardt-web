use bytes::Bytes;
use http::{HeaderMap, Method, StatusCode, Uri, Version};
use reinhardt_exception::Result;
use reinhardt_http::{Request, Response};
use reinhardt_integration_tests::test_helpers::{shutdown_test_server, spawn_test_server};
use reinhardt_types::Handler;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// REST API handler for a simple blog
struct BlogApiHandler {
    posts: Arc<Mutex<HashMap<u32, BlogPost>>>,
    next_id: Arc<Mutex<u32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BlogPost {
    id: u32,
    title: String,
    content: String,
    author: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreatePostRequest {
    title: String,
    content: String,
    author: String,
}

impl BlogApiHandler {
    fn new() -> Self {
        let mut posts = HashMap::new();
        posts.insert(
            1,
            BlogPost {
                id: 1,
                title: "First Post".to_string(),
                content: "Hello, World!".to_string(),
                author: "Admin".to_string(),
            },
        );

        Self {
            posts: Arc::new(Mutex::new(posts)),
            next_id: Arc::new(Mutex::new(2)),
        }
    }
}

#[async_trait::async_trait]
impl Handler for BlogApiHandler {
    async fn handle(&self, request: Request) -> Result<Response> {
        let path = request.path();
        let method = &request.method;

        match (method.as_str(), path) {
            // GET /api/posts - List all posts
            ("GET", "/api/posts") => {
                let posts = self.posts.lock().unwrap();
                let posts_vec: Vec<_> = posts.values().cloned().collect();
                let json = serde_json::to_string(&posts_vec)
                    .map_err(|e| reinhardt_exception::Error::Serialization(e.to_string()))?;
                Ok(Response::ok()
                    .with_header("content-type", "application/json")
                    .with_body(json))
            }

            // GET /api/posts/{id} - Get single post
            ("GET", path) if path.starts_with("/api/posts/") => {
                let id_str = path.trim_start_matches("/api/posts/");
                match id_str.parse::<u32>() {
                    Ok(id) => {
                        let posts = self.posts.lock().unwrap();
                        match posts.get(&id) {
                            Some(post) => {
                                let json = serde_json::to_string(post).map_err(|e| {
                                    reinhardt_exception::Error::Serialization(e.to_string())
                                })?;
                                Ok(Response::ok()
                                    .with_header("content-type", "application/json")
                                    .with_body(json))
                            }
                            None => Ok(Response::not_found()
                                .with_header("content-type", "application/json")
                                .with_body(r#"{"error":"Post not found"}"#)),
                        }
                    }
                    Err(_) => Ok(Response::new(StatusCode::BAD_REQUEST)
                        .with_header("content-type", "application/json")
                        .with_body(r#"{"error":"Invalid post ID"}"#)),
                }
            }

            // POST /api/posts - Create new post
            ("POST", "/api/posts") => {
                let body = request.read_body()?;
                let body_str = String::from_utf8(body.to_vec())
                    .map_err(|e| reinhardt_exception::Error::ParseError(e.to_string()))?;
                let create_req: CreatePostRequest = serde_json::from_str(&body_str)
                    .map_err(|e| reinhardt_exception::Error::Serialization(e.to_string()))?;

                let mut next_id = self.next_id.lock().unwrap();
                let id = *next_id;
                *next_id += 1;

                let post = BlogPost {
                    id,
                    title: create_req.title,
                    content: create_req.content,
                    author: create_req.author,
                };

                let mut posts = self.posts.lock().unwrap();
                posts.insert(id, post.clone());

                let json = serde_json::to_string(&post)
                    .map_err(|e| reinhardt_exception::Error::Serialization(e.to_string()))?;
                Ok(Response::new(StatusCode::CREATED)
                    .with_header("content-type", "application/json")
                    .with_body(json))
            }

            // PUT /api/posts/{id} - Update post
            ("PUT", path) if path.starts_with("/api/posts/") => {
                let id_str = path.trim_start_matches("/api/posts/");
                match id_str.parse::<u32>() {
                    Ok(id) => {
                        let body = request.read_body()?;
                        let body_str = String::from_utf8(body.to_vec())
                            .map_err(|e| reinhardt_exception::Error::ParseError(e.to_string()))?;
                        let update_req: CreatePostRequest = serde_json::from_str(&body_str)
                            .map_err(|e| {
                                reinhardt_exception::Error::Serialization(e.to_string())
                            })?;

                        let mut posts = self.posts.lock().unwrap();
                        if posts.contains_key(&id) {
                            let post = BlogPost {
                                id,
                                title: update_req.title,
                                content: update_req.content,
                                author: update_req.author,
                            };
                            posts.insert(id, post.clone());

                            let json = serde_json::to_string(&post).map_err(|e| {
                                reinhardt_exception::Error::Serialization(e.to_string())
                            })?;
                            Ok(Response::ok()
                                .with_header("content-type", "application/json")
                                .with_body(json))
                        } else {
                            Ok(Response::not_found()
                                .with_header("content-type", "application/json")
                                .with_body(r#"{"error":"Post not found"}"#))
                        }
                    }
                    Err(_) => Ok(Response::new(StatusCode::BAD_REQUEST)
                        .with_header("content-type", "application/json")
                        .with_body(r#"{"error":"Invalid post ID"}"#)),
                }
            }

            // DELETE /api/posts/{id} - Delete post
            ("DELETE", path) if path.starts_with("/api/posts/") => {
                let id_str = path.trim_start_matches("/api/posts/");
                match id_str.parse::<u32>() {
                    Ok(id) => {
                        let mut posts = self.posts.lock().unwrap();
                        if posts.remove(&id).is_some() {
                            Ok(Response::new(StatusCode::NO_CONTENT))
                        } else {
                            Ok(Response::not_found()
                                .with_header("content-type", "application/json")
                                .with_body(r#"{"error":"Post not found"}"#))
                        }
                    }
                    Err(_) => Ok(Response::new(StatusCode::BAD_REQUEST)
                        .with_header("content-type", "application/json")
                        .with_body(r#"{"error":"Invalid post ID"}"#)),
                }
            }

            _ => Ok(Response::not_found()
                .with_header("content-type", "application/json")
                .with_body(r#"{"error":"Endpoint not found"}"#)),
        }
    }
}

#[tokio::test]
async fn test_e2e_list_posts() {
    let handler = Arc::new(BlogApiHandler::new());
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/api/posts", url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let posts: Vec<BlogPost> = response.json().await.unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0].title, "First Post");

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_e2e_get_single_post() {
    let handler = Arc::new(BlogApiHandler::new());
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/api/posts/1", url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let post: BlogPost = response.json().await.unwrap();
    assert_eq!(post.id, 1);
    assert_eq!(post.title, "First Post");

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_e2e_get_nonexistent_post() {
    let handler = Arc::new(BlogApiHandler::new());
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/api/posts/999", url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 404);

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_e2e_create_post() {
    let handler = Arc::new(BlogApiHandler::new());
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();
    let new_post = CreatePostRequest {
        title: "Second Post".to_string(),
        content: "This is the second post".to_string(),
        author: "User".to_string(),
    };

    let response = client
        .post(&format!("{}/api/posts", url))
        .json(&new_post)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 201);
    let created_post: BlogPost = response.json().await.unwrap();
    assert_eq!(created_post.id, 2);
    assert_eq!(created_post.title, "Second Post");

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_e2e_update_post() {
    let handler = Arc::new(BlogApiHandler::new());
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();
    let update_data = CreatePostRequest {
        title: "Updated Title".to_string(),
        content: "Updated content".to_string(),
        author: "Admin".to_string(),
    };

    let response = client
        .put(&format!("{}/api/posts/1", url))
        .json(&update_data)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let updated_post: BlogPost = response.json().await.unwrap();
    assert_eq!(updated_post.title, "Updated Title");

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_e2e_delete_post() {
    let handler = Arc::new(BlogApiHandler::new());
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // Delete the post
    let response = client
        .delete(&format!("{}/api/posts/1", url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 204);

    // Verify it's gone
    let response = client
        .get(&format!("{}/api/posts/1", url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_e2e_full_crud_workflow() {
    let handler = Arc::new(BlogApiHandler::new());
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // 1. List initial posts
    let response = client
        .get(&format!("{}/api/posts", url))
        .send()
        .await
        .unwrap();
    let posts: Vec<BlogPost> = response.json().await.unwrap();
    assert_eq!(posts.len(), 1);

    // 2. Create a new post
    let new_post = CreatePostRequest {
        title: "Test Post".to_string(),
        content: "Test content".to_string(),
        author: "Tester".to_string(),
    };
    let response = client
        .post(&format!("{}/api/posts", url))
        .json(&new_post)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 201);
    let created: BlogPost = response.json().await.unwrap();
    let post_id = created.id;

    // 3. Read the created post
    let response = client
        .get(&format!("{}/api/posts/{}", url, post_id))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let fetched: BlogPost = response.json().await.unwrap();
    assert_eq!(fetched.title, "Test Post");

    // 4. Update the post
    let update_data = CreatePostRequest {
        title: "Updated Test Post".to_string(),
        content: "Updated content".to_string(),
        author: "Tester".to_string(),
    };
    let response = client
        .put(&format!("{}/api/posts/{}", url, post_id))
        .json(&update_data)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let updated: BlogPost = response.json().await.unwrap();
    assert_eq!(updated.title, "Updated Test Post");

    // 5. Delete the post
    let response = client
        .delete(&format!("{}/api/posts/{}", url, post_id))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 204);

    // 6. Verify deletion
    let response = client
        .get(&format!("{}/api/posts/{}", url, post_id))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_e2e_concurrent_operations() {
    let handler = Arc::new(BlogApiHandler::new());
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // Create multiple posts concurrently
    let mut handles = vec![];
    for i in 0..5 {
        let client = client.clone();
        let url = url.clone();
        let h = tokio::spawn(async move {
            let new_post = CreatePostRequest {
                title: format!("Concurrent Post {}", i),
                content: format!("Content {}", i),
                author: "Tester".to_string(),
            };
            client
                .post(&format!("{}/api/posts", url))
                .json(&new_post)
                .send()
                .await
                .unwrap()
        });
        handles.push(h);
    }

    // Wait for all to complete
    for h in handles {
        let response = h.await.unwrap();
        assert_eq!(response.status(), 201);
    }

    // Verify all posts were created
    let response = client
        .get(&format!("{}/api/posts", url))
        .send()
        .await
        .unwrap();
    let posts: Vec<BlogPost> = response.json().await.unwrap();
    assert_eq!(posts.len(), 6); // 1 initial + 5 new

    shutdown_test_server(handle).await;
}
