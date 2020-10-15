mod tg_helpers;

use std::sync::{Arc, Mutex};
use std::{thread, time, fs};

use rtdlib::types::*;
use telegram_client::api::Api;
use telegram_client::client::Client;

use tg_helpers::get_tg_user;


fn load_configuration(path: &str) -> (i64, String, i64) {
    let raw_conf = fs::read_to_string(path).expect("Could not read configuration file!");
    let conf: serde_json::Value = serde_json::from_str(&raw_conf).expect("Configuration file is malformed");

    let api_id = conf["api_id"].as_i64().expect("Configuration file does not contain an API_ID!");
    let api_hash = conf["api_hash"].as_str().expect("Configuration file does not contain an API_HASH!").to_string();
    let output_verbosity = conf["output_verbosity"].as_i64().expect("Configuration file does not contain an output_verbosity!");

    return (api_id.clone(), api_hash.clone(), output_verbosity);
}


fn main() {
    let (_, _, output_verbosity) = load_configuration("configuration.json");

    let _ = Client::set_log_verbosity_level(1);

    let api = Api::default();
    let mut client = Client::new(api.clone());
    let listener = client.listener();

    let have_authorization: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));

    listener.on_update_option(|(_api, option)| {
        let value = option.value();

        match &option.name()[..] {
            "version" => { value.as_string().map(|v| println!("VERSION IS {}", v.value())); }
            _ => {}
        };
        Ok(())
    });

    listener.on_update_authorization_state(move |(api, update)| {
        let state = update.authorization_state();
        state.on_wait_tdlib_parameters(move |_| {
            let (api_id, api_hash, _) = load_configuration("configuration.json");
            let paras = SetTdlibParameters::builder()
                .parameters(TdlibParameters::builder()
                    .database_directory("tdlib")
                    .use_message_database(true)
                    .use_secret_chats(true)
                    .api_id(api_id)
                    .api_hash(api_hash)
                    .system_language_code("en")
                    .device_model("Server")
                    .system_version("Unknown")
                    .application_version(env!("CARGO_PKG_VERSION"))
                    .enable_storage_optimizer(true)
                    .build())
                .build();
            let _ = api.send(&paras);
        });

        state.on_wait_encryption_key(|_| {
            let _ = api.send(CheckDatabaseEncryptionKey::builder().build());
        });
        state.on_wait_phone_number(|_| {
            let _ = api.send(CheckAuthenticationBotToken::builder().token("1355835771:AAHY5-Fpi44l0L0xdQ-3oA8JUCplLpCUi5w").build());
        });
        state.on_ready(|_| {
            let mut have_authorization = have_authorization.lock().unwrap();
            *have_authorization = true;
            println!("Authorization ready");
        });
        state.on_logging_out(|_| {
            let mut have_authorization = have_authorization.lock().unwrap();
            *have_authorization = false;
            println!("Logging out");
        });
        state.on_closing(|_| {
            let mut have_authorization = have_authorization.lock().unwrap();
            *have_authorization = false;
            println!("Closing");
        });
        state.on_closed(|_| {
            println!("Closed");
        });
        Ok(())
    });

    listener.on_update_connection_state(|(_api, update)| {
        let state = update.state();
        state
            .on_waiting_for_network(|_| { println!("Waiting for network..."); })
            .on_connecting(|_| { println!("Connecting"); })
            .on_updating(|_| { println!("Updating..."); })
            .on_ready(|_| { println!("Connection ready") });
        Ok(())
    });

    listener.on_error(|(_api, error)| {
        let code = error.code();
        let message = error.message();
        println!("ERROR ({}): {}", code, message);
        Ok(())
    });

    listener.on_user(|(_api, user)| {
        let user_c = user.clone();
        let _ = thread::spawn(move || {     // New thread, so other threads can finish their job and we don't get deadlocked
            unsafe {                                                // See explanation for this in get_tg_user
                while tg_helpers::REQUESTED_USER.is_some() {          // Trying to prevent overwriting an unused result
                    thread::sleep(time::Duration::from_millis(10)); // This should be done with mutexes, not like this!
                }
                tg_helpers::REQUESTED_USER = Some(user_c.clone());
            }
        });
        Ok(())
    });

    if output_verbosity > 4 {
        listener.on_receive(|(_api, json)| {
            println!("{}", json);
            Ok(())
        });
    }



    listener.on_update_new_message(|(api, update)| {
        let message : Message = update.message().clone();
        let api_c = api.clone();

        // We have to start a new thread because all listener functions will run on the same thread
        // This causes us to not be able to get any information from the api while handling anything
        let _ = thread::spawn(move || {
            let content = message.content();
            content.on_message_text(|m| {
                let sender = get_tg_user(&api_c, message.sender_user_id());
                println!("New message from {} {}: \"{}\"", sender.first_name(), sender.last_name(), m.text().text());
            });

        });

        Ok(())
    });



    let _ = client.daemon("telegram-rs");
}
