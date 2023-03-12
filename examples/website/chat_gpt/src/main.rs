use perseus::prelude::*;
use serde::{Deserialize, Serialize};
use sycamore::prelude::*;

#[cfg(client)]
enum GptModel {
    V4,
    V3_5,
}

// Initialize our app with the `perseus_warp` package's default server (fullay
// customizable)
#[perseus::main(perseus_axum::dflt_server)]
pub fn main<G: Html>() -> PerseusApp<G> {
    PerseusApp::new()
        .plugins(Plugins::new().plugin(
            perseus_tailwind::get_tailwind_plugin,
            perseus_tailwind::TailwindOptions {
                in_file: "src/tailwind.css".into(),
                // Don't put this in /static, it will trigger build loops.
                // Put this in /dist and use a static alias instead.
                out_file: "dist/tailwind.css".into(),
            },
        ))
        .static_alias("/tailwind.css", "dist/tailwind.css")        
        // Create a new template at `index`, which maps to our landing page
        .template(
            Template::build("index")
                .view_with_state(index_page)
                .build_state_fn(get_build_state)
                .build(),
        )
        .template(Template::build("about").view(about_page).build())
}

#[cfg(client)]
fn to_json_str(ls: Vec<ChatSnippet>) -> String {
    // returns "[{"role": "user", ..}, ..]"
    let result = ls.iter().map(|snippet| {
        let x = snippet.role.to_string();
        let y = snippet.text.to_string();
        format!(r#"{{"role": "{x}", "content": "{y}"}}"#)
    }).collect::<Vec<String>>().join(", ");
    return format!("[{}]", result)
}

#[cfg(client)]
async fn ask_chatgpt(chat: Vec<ChatSnippet>, api_key: String, model: GptModel, temperature: f32) -> String {
    let chat = to_json_str(chat);
    let model_str = (match model {
        GptModel::V3_5 => "gpt-3.5-turbo",
        GptModel::V4 => "gpt-4",
    }).to_string();

    let post_body = format!(r#"{{"model": "{model_str}", "messages": {chat}, "temperature": {temperature}}}"#);

    let response = reqwasm::http::Request::post("https://api.openai.com/v1/chat/completions")
            .body(post_body)
                .header("Authorization", &*format!("Bearer {api_key}")).header("Content-Type", "application/json").send().await.unwrap();
    let text_result = response.text().await;
    let result = text_result.unwrap_or("{}".to_string());
    let s1 = serde_json::from_str(&result).unwrap_or(serde_json::Value::Null)["choices"][0]["message"]["content"].to_string();
    return (&s1[1..s1.len()-1]).to_string();
}

fn ApiKeySettings<'a, G: Html>(
    cx: BoundedScope<'_, 'a>,
    state: &'a IndexStateRx,
) -> View<G> {
    view! { cx,
        span {"API Key"} input(bind:value=state.api_key)
        button(
            on:click =move|_| {
                #[cfg(client)]
                spawn_local_scoped(cx, async {
                    use gloo_storage::{Storage, LocalStorage};
                    LocalStorage::set("api_key", &*state.api_key.get());
                });
            },
        ){"Save"}     
    }
}

#[cfg(client)]
fn load_storage<'a>(state: &'a IndexStateRx) {
    use gloo_storage::{Storage, LocalStorage};
    state.api_key.set(LocalStorage::get("api_key").unwrap());
}

#[cfg(client)]
fn ask_chatgpt_question<'a>(cx: BoundedScope<'_, 'a>, state: &'a IndexStateRx) {
    spawn_local_scoped(cx, async {
        use gloo_storage::{Storage, LocalStorage};
        let user_question = state.current_question.get().to_string();
        let mut ongoing_chat = (*state.chat.get()).clone();
        ongoing_chat.push(ChatSnippet{text: user_question, role: "user".to_string()});
        state.chat.set(ongoing_chat.clone());  
        let result = ask_chatgpt(ongoing_chat.clone(), (*state.api_key.get()).clone(), GptModel::V3_5, 0.7)
                        .await
                        .replace('\n', "\n");
        ongoing_chat.push(ChatSnippet{text: result, role: "assistant".to_string()});
        state.chat.set(ongoing_chat.clone());
        let opening_question = format!("chat-{}",ongoing_chat[0].text);
        LocalStorage::set(opening_question, ongoing_chat).unwrap();
    });
}


// #[auto_scope]
// EXCERPT_START
fn index_page<'a, G: Html>(
    cx: BoundedScope<'_, 'a>,
    state: &'a IndexStateRx,
) -> View<G> {
    #[cfg(client)] load_storage(state);

    view! { cx,
        h1 { "ChatGPT Assistant" } 
        ApiKeySettings(state)
        br()br()br()
        button(on:click =move|_| { #[cfg(client)] spawn_local_scoped(cx, async { state.chat.set(vec![]) }); },){"Reset Chat"}
        br()
        input(bind:value=state.current_question)
        button(on:click =move|_| { #[cfg(client)] ask_chatgpt_question(cx, state) },){"Click me"}
        ul {
            Indexed(
                iterable= &state.chat,
                view=move |cx, snippet| view! { cx,
                    li { (snippet.text) }
                    br()
                },
            )
        }
        input(
            value = "text",
            readonly = true,
            style = "border: 1px solid gray; background-color: #f0f0f0; font-size: 16px;",
        )
        br()
        a(href = "about") { "About" }
    }
}

// #[browser_only_fn]
// async fn greeting_handler<'a>(
//     _cx: Scope<'a>,
//     greeting: &'a RcSignal<Result<String, SerdeInfallible>>,
// ) -> Result<(), SerdeInfallible> {
//     greeting.set(Ok(LocalStorage::get("api_key").unwrap()));
//     Ok(())
// }

#[derive(Serialize, Deserialize, Clone)]
struct ChatSnippet {
    text: String,
    role: String,
}

#[derive(Serialize, Deserialize, Clone, ReactiveState)]
#[rx(alias = "IndexStateRx")]
struct IndexState {
    chat: Vec<ChatSnippet>,
    current_question: String,
    // #[rx(suspense = "greeting_handler")]
    api_key: String,
}
impl PartialEq for ChatSnippet {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text && self.role == other.role
    }
}
// This function will be run when you build your app, to generate default state
// ahead-of-time
#[engine_only_fn]
async fn get_build_state(_info: StateGeneratorInfo<()>) -> IndexState {
    IndexState {
        chat: vec![],
        current_question: "".to_string(),
        api_key: "".to_string()
        // api_key: Ok("".to_string())
    }
}
// EXCERPT_END

fn about_page<G: Html>(cx: Scope) -> View<G> {
    view! { cx,
        p { "This is an example webapp created with Perseus!" }
    }
}
