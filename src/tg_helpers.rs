use std::{thread, time};

use rtdlib::types::*;
use telegram_client::api::Api;

pub static mut REQUESTED_USER: Option<User> = None;     // Global Variable to temporarily save a requested user

pub fn get_tg_user(api : &Api, userid : i64) -> User{
    let _ = api.get_user(GetUser::builder().user_id(userid).build());

    let mut counter = 0;

    // This unsafe tomfoolery is a temporary fix since the author of telegram-client/rtdlib is working on things being awaitable
    // This is absolutely bad practice and will probably crash and burn in some edge cases but it's enough for now
    unsafe {
        while counter < 200 {                           // If we haven't gotten an answer after 2 seconds we're probably not getting one at all
            if let Some(r_user) = &REQUESTED_USER {
                if r_user.id() == userid {
                    let user = r_user.clone();
                    REQUESTED_USER = None;
                    return user;
                }
            }
            counter += 1;
            thread::sleep(time::Duration::from_millis(10));
        }
    }

    return User::builder().first_name("Unknown").last_name("User").id(-1).build();
}