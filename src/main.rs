use itertools::Itertools;
use rocket::data::{self, Data, FromData, ToByteUnit};
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::{self, Request};
use rocket::response::content;
use std::ops::Not;
use std::num::ParseIntError;

#[macro_use]
extern crate rocket;

#[derive(Debug)]
enum Error {
    InvalInput,
    TooLarge,
    Io(std::io::Error),
}

impl From<ParseIntError> for Error {
    fn from(_: ParseIntError) -> Error {
        Error::InvalInput
    }
}

#[derive(Debug)]
struct GroupFencerResult {
    name: String,
    wins: u8,
    given: u8,
    taken: u8,
}

impl GroupFencerResult {
    fn to_form_entry(self: &Self) -> String {
        let mut out_str = String::new();

        out_str.push_str("<div>\n");
        out_str.push_str(&format!("<lable>{}:</lable>\n", self.name));
        out_str.push_str(&format!(
            "<input name=\"{}_wins\" value=\"{}\">\n",
            self.name, self.wins
        ));
        out_str.push_str(&format!(
            "<input name=\"{}_given\" value=\"{}\">\n",
            self.name, self.given
        ));
        out_str.push_str(&format!(
            "<input name=\"{}_taken\" value=\"{}\">\n",
            self.name, self.taken
        ));
        out_str.push_str("</div>\n");

        out_str
    }
}

impl GroupFencerResult {
    fn from_hashmap(name: &str, map: &std::collections::HashMap<&str, u8>) -> GroupFencerResult {
        GroupFencerResult {
            name: name.to_owned(),
            wins: map[&(name.to_owned()+"_wins") as &str],
            given: map[&(name.to_owned()+"_given") as &str],
            taken: map[&(name.to_owned()+"_taken") as &str],
            }
    }
}

struct Group {
    results: std::collections::HashMap<String, GroupFencerResult>,
}

fn check_all_entries(
    map: &std::collections::HashMap<&str, u8>,
    fancers: &[&str],
    exts: &[&str],
) -> Result<(), Error> {
    for fancer in fancers {
        for ext in exts {
            let mut name = fancer.to_string();
            name.push_str(ext);
            if map.contains_key(&name as &str).not() {
                return Err(Error::InvalInput);
            }
        }
    }
    Ok(())
}

fn data_into_hashmap(data: &str) -> Result<std::collections::HashMap<&str, u8>, Error> {
    let mut map = std::collections::HashMap::new();
    let data_items = data.split("&");

    for item in data_items {
        let mut data_iter = item.split("=");
        let Some(key) = data_iter.next() else {
            return Err(Error::InvalInput);
        };
        let Some(value) = data_iter.next() else {
            return Err(Error::InvalInput);
        };

        map.insert(
            key,
            value.parse::<u8>()?,
        );
    }
    Ok(map)
}

#[rocket::async_trait]
impl<'a> FromData<'a> for Group {
    type Error = Error;

    async fn from_data(req: &'a Request<'_>, data: Data<'a>) -> data::Outcome<'a, Self> {
        use Error::*;

        let data = match data.open(1024.bytes()).into_string().await {
            Ok(string) if string.is_complete() => string.into_inner(),
            Ok(_) => return Outcome::Error((Status::PayloadTooLarge, TooLarge)),
            Err(e) => return Outcome::Error((Status::InternalServerError, Io(e))),
        };

        let Ok(map) = data_into_hashmap(&data) else {
            return Outcome::Error((Status::InternalServerError, InvalInput));
        };

        let fancers: Vec<_> = map
            .keys()
            .map(|key| key.split("_").next().unwrap())
            .unique()
            .collect();

        if let Err(error) = check_all_entries(&map, &fancers, &["_wins", "_taken", "_given"]) {
            return Outcome::Error((Status::InternalServerError, error));
        }

        let mut results = std::collections::HashMap::new();
        for fancer in fancers {
            let res = GroupFencerResult::from_hashmap(fancer, &map);
            results.insert(fancer.to_string(), res);
        }

        println!("{:?}", results);

        data::Outcome::Success(Group { results})
    }
}

#[get("/")]
fn index() -> content::RawHtml<String> {
    let res = GroupFencerResult {
        name: "Johann".into(),
        wins: 0,
        given: 0,
        taken: 0,
    };

    let mut form = String::new();
    form.push_str(r#"<form action="/data_in" method="post">"#);
    form.push_str(&res.to_form_entry());
    form.push_str(r#"<input type="submit" value="Submit">"#);
    form.push_str("</form>");

    content::RawHtml(form)
}

#[post("/data_in", data = "<input>")]
fn data_in(input: Group) {
    println!("{:?}", input.results);
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/", routes![data_in])
}
