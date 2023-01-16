use std::{borrow::Cow, env};

use rocket::{
    fs::{relative, FileServer},
    launch,
};

#[launch]
async fn rocket() -> _ {
    let static_dir = env::var("SIGNAL_INSPECTOR_STATIC_DIR")
        .map(Cow::Owned)
        .unwrap_or_else(|_| relative!("../frontend/dist/").into());
    rocket::build().mount("/", FileServer::from(&*static_dir))
}
