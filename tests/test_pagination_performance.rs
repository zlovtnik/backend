use reqwest;
use serde_json::json;
use std::time::Instant;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = "http://localhost:8000/api";
    let client = reqwest::Client::new();

    // First, get a JWT token
    println!("🔐 Getting JWT token...");
    let login_response = client
        .post(&format!("{}/auth/login", base_url))
        .json(&json!({
            "username": "admin",
            "password": "password"
        }))
        .send()
        .await?;

    if !login_response.status().is_success() {
        println!(
            "❌ Failed to login. Make sure your API server is running and credentials are correct."
        );
        return Ok(());
    }

    let login_data: serde_json::Value = login_response.json().await?;
    let token = login_data["data"]["token"].as_str().unwrap();

    println!("✅ Successfully authenticated");

    // Test 1: First page performance
    println!("\n📊 Testing pagination performance with 1M records...");

    let start = Instant::now();
    let response = client
        .get(&format!("{}/address-book?page=1&per_page=50", base_url))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    let duration = start.elapsed();

    if response.status().is_success() {
        let data: serde_json::Value = response.json().await?;
        println!("✅ First page (1-50): {:?}", duration);
        println!(
            "   Total records: {}",
            data["data"]["total"].as_u64().unwrap_or(0)
        );
        println!(
            "   Records returned: {}",
            data["data"]["data"].as_array().unwrap().len()
        );
    } else {
        println!("❌ First page request failed: {}", response.status());
    }

    // Test 2: Middle page performance (page 10,000)
    let start = Instant::now();
    let response = client
        .get(&format!("{}/address-book?page=10000&per_page=50", base_url))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    let duration = start.elapsed();

    if response.status().is_success() {
        let data: serde_json::Value = response.json().await?;
        println!("✅ Middle page (10,000): {:?}", duration);
        println!(
            "   Records returned: {}",
            data["data"]["data"].as_array().unwrap().len()
        );
    } else {
        println!("❌ Middle page request failed: {}", response.status());
    }

    // Test 3: Search performance
    let start = Instant::now();
    let response = client
        .get(&format!(
            "{}/address-book?page=1&per_page=50&name=John",
            base_url
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    let duration = start.elapsed();

    if response.status().is_success() {
        let data: serde_json::Value = response.json().await?;
        println!("✅ Search by name 'John': {:?}", duration);
        println!(
            "   Matching records: {}",
            data["data"]["total"].as_u64().unwrap_or(0)
        );
    } else {
        println!("❌ Search request failed: {}", response.status());
    }

    // Test 4: Age filter performance
    let start = Instant::now();
    let response = client
        .get(&format!(
            "{}/address-book?page=1&per_page=50&min_age=25&max_age=35",
            base_url
        ))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    let duration = start.elapsed();

    if response.status().is_success() {
        let data: serde_json::Value = response.json().await?;
        println!("✅ Age filter (25-35): {:?}", duration);
        println!(
            "   Matching records: {}",
            data["data"]["total"].as_u64().unwrap_or(0)
        );
    } else {
        println!("❌ Age filter request failed: {}", response.status());
    }

    // Test 5: Stress test - multiple concurrent requests
    println!("\n🚀 Running stress test (10 concurrent requests)...");
    let start = Instant::now();

    let mut handles = vec![];
    for i in 1..=10 {
        let client = client.clone();
        let token = token.to_string();
        let base_url = base_url.to_string();

        let handle = tokio::spawn(async move {
            let response = client
                .get(&format!(
                    "{}/address-book?page={}&per_page=50",
                    base_url,
                    i * 100
                ))
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await;

            match response {
                Ok(resp) => resp.status().is_success(),
                Err(_) => false,
            }
        });

        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;
    let successful = results
        .iter()
        .filter(|r| *r.as_ref().unwrap_or(&false))
        .count();
    let duration = start.elapsed();

    println!("✅ Stress test completed: {:?}", duration);
    println!("   Successful requests: {}/10", successful);

    println!("\n📈 Performance Summary:");
    println!("   • First page: Very fast (< 100ms expected)");
    println!("   • Middle pages: Slower due to OFFSET (> 100ms expected)");
    println!("   • Search queries: Depends on indexes");
    println!("   • Recommendation: Implement cursor-based pagination for better performance");

    Ok(())
}
