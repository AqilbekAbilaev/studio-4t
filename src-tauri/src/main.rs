// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// use mongodb::{
//     bson::{doc, Document},
//     sync::{Client, Collection},
// };

fn main() {
    // std::env::set_var("GTK_OVERLAY_SCROLLING", "0");
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    studio_4t_lib::run()
}

// static URI: &str = "mongodb://test:test@localhost:27017/?authSource=admin";
// struct DbConnection {
//     client: Client,
// }

// fn db() -> mongodb::error::Result<()> {
//     // Replace the placeholder with your Atlas connection string

//     let client = Client::with_uri_str(uri)?;
//     // Create a new client and connect to the server

//     // Get a handle on the movies collection
//     let database = client.database("rust");
//     let my_coll: Collection<Document> = database.collection("users");

//     // Find a movie based on the title value
//     let my_movie = my_coll
//         .find_one(doc! { "name": "shit" })
//         .run()?;

//     // Print the document
//     println!("Found a user:\n{:#?}", my_movie);
//     Ok(())
// }

// const CONNE: DbConnection = DbConnection {
//     client: Client::with_uri_str(URI)?,
// };