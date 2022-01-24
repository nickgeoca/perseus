mod about;
mod global_state;
mod idb;
mod index;
mod test;

use perseus::define_app;

define_app! {
    templates: [
        index::get_template::<G>(),
        test::get_template::<G>(),
        about::get_template::<G>(),
        idb::get_template::<G>()
    ],
    error_pages: perseus::ErrorPages::new(|url, status, err, _| {
        sycamore::view! {
            p { (format!("An error with HTTP code {} occurred at '{}': '{}'.", status, url, err)) }
        }
    }),
    global_state_creator: global_state::get_global_state_creator()
}