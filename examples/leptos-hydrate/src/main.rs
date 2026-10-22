#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos::prelude::*;
    use leptos_axum::{LeptosRoutes, file_and_error_handler, generate_route_list};
    use leptos_hydrate::{App, shell};
    use tokio::net::TcpListener;

    let configuration = get_configuration(None).expect("Leptos configuration");
    let options = configuration.leptos_options;
    let address = options.site_addr;
    let routes = generate_route_list(App);
    let app = Router::new()
        .leptos_routes(&options, routes, {
            let options = options.clone();
            move || shell(options.clone())
        })
        .fallback(file_and_error_handler(shell))
        .with_state(options);

    let listener = TcpListener::bind(&address)
        .await
        .expect("bind site address");
    println!("listening on http://{address}");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("serve");
}

#[cfg(not(feature = "ssr"))]
fn main() {}
