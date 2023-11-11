use actix::prelude::*;
use actix_web::{post, web, App, HttpResponse, HttpServer};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
struct AskPrompt {
    prompt: String,
}

struct ChatContext {
    context: String,
}

impl Actor for ChatContext {
    type Context = Context<Self>;
}

impl ChatContext {
    fn new() -> Self {
        Self {
            context: String::new(),
        }
    }
}

#[derive(Message)]
#[rtype(result = "String")]
struct GetContext;

#[derive(Message)]
#[rtype(result = "()")]
struct SetContext(String);

impl Handler<GetContext> for ChatContext {
    type Result = String;

    fn handle(&mut self, _msg: GetContext, _ctx: &mut Context<Self>) -> Self::Result {
        self.context.clone()
    }
}

impl Handler<SetContext> for ChatContext {
    type Result = ();

    fn handle(&mut self, msg: SetContext, _ctx: &mut Context<Self>) -> Self::Result {
        self.context = msg.0;
    }
}

#[post("/chat")]
async fn chat(info: web::Json<AskPrompt>, context: web::Data<Addr<ChatContext>>) -> HttpResponse {
    let context_addr = context.get_ref().clone();
    let context = context_addr.send(GetContext).await.unwrap();

    let prompt: AskPrompt = AskPrompt {
        prompt: format!("{} {}", context, info.prompt),
    };

    let prompt_to_json: String = serde_json::to_string(&prompt).unwrap();

    let client: reqwest::Client = reqwest::Client::new();

    let res: Result<reqwest::Response, reqwest::Error> = client
        .post("http://localhost:11434/api/generate")
        .json(&serde_json::json!({
            "model": "llama2:7b",
            "prompt": prompt_to_json,
            "stream": false,
        }))
        .send()
        .await;

    if let Ok(response) = res {
        let body: String = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read response body".to_string());

        let body_parsed_to_json: Value = serde_json::from_str(&body).unwrap();
        let get_response_from_body: Option<&str> = body_parsed_to_json["response"].as_str();

        context_addr.do_send(SetContext(String::from(get_response_from_body.unwrap())));

        return HttpResponse::Ok().body(String::from(get_response_from_body.unwrap()));
    }

    HttpResponse::InternalServerError().body("Error")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let context = ChatContext::new().start();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(context.clone()))
            .service(chat)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
