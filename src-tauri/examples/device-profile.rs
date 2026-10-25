#[tokio::main]
async fn main() {
    let profile = vidextract_lib::hardware::inspect_device()
        .await
        .expect("device inspection");
    println!("{}", serde_json::to_string_pretty(&profile).unwrap());
}
