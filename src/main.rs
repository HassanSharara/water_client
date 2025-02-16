use water_client::models::client::{HttpClient};
use water_client::models::request::{ HttpRequest};

#[tokio::main]
async fn main(){
  let mut client = HttpClient::new("https://google.com".into());
    client.init_connection().await;

    loop {
        let   request = HttpRequest::get("/");
        match   client.send_request(
            request
        ).await {
            Ok(res) => {
                    let response = res.get_full_body_bytes().await;
                    if let Ok(ref body ) = response {
                        println!("success {:?}",body.len());
                        continue;
                    }
                    println!("error");
            }
            Err(e) => {
                println!("err {:?}",e);
            }
        }
    }
}