use serde_json::json;
use water_client::models::client::{HttpClient, SendingRequestError};
use water_client::models::request::{HttpBody, HttpRequest};

#[tokio::main]
async fn main(){
  let mut client = HttpClient::new("127.0.0.1:8084".into());
    client.init_connection().await;


    loop {
        let mut request = HttpRequest::post("/");
        request.set_body(
            HttpBody::from_json(
                &json!({
                "hello":"world"
            })
            )
        );

        match client.send_request(
            request
        ).await {
            Ok(response) => { println!("request sent successfully");
                let body = response.get_full_body_bytes().await;
                if let Ok(body) =body {
                    continue;
                }
                println!("error invoked");
            }
            Err(e) => {
                println!("error {:?}",e);
            }
        }


    }
}