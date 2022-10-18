use nickel::{Request, Response, MiddlewareResult};

fn logger_fn<'mw>(req: &mut Request, res: Response<'mw>) -> MiddlewareResult<'mw> {
    println!("logging request from logger fn: {:?}", req.origin.uri);
    res.next_middleware()
}

mod rest_api {
    use std::sync::{Arc, Mutex};

    use nickel::{status::StatusCode, *};
    use rustorm::Pool;

    use crate::logger_fn;

    mod json_structs {
        use serde::Serialize;

        #[derive(Debug, Serialize)]
        pub struct SensorInfo {
            pub id: i32,
            pub group_id: i32,
            pub r#type: String,
            pub value: i32,
        }
    }

    mod for_retrieve {
        use rustorm::*;
        use serde::Serialize;

        #[derive(Serialize, Debug, FromDao, ToColumnNames, ToTableName)]
        pub struct Sensor {
            pub id: i32,
            pub addr: i32,
            pub name: String,
            pub description: String,
            pub writable: bool,
            pub bits: String,
        }

        #[derive(Serialize, Debug, FromDao, ToColumnNames, ToTableName)]
        pub struct SensorQuery {
            pub id: i32,
            pub group_id: i32,
            pub bits: String,
            pub value: i32,
            pub writable: bool,
        }
    }

    pub fn main() -> Result<(), String> {
        let mut server = Nickel::new();
        let mut pool = Pool::new();

        let real_db = match pool.em("sqlite://test.sqlite") {
            Ok(it) => it,
            Err(error) => return Err(error.to_string()),
        };

        let db_mutex = Mutex::new(real_db);
        let db = Arc::new(db_mutex);

        server.utilize(logger_fn);

        let db_sensors_id = db.clone();
        server.get(
            "/sensors/:id",
            middleware! {
                |request|

                println!("{:?}", request.query());

                let result = if let Some(id_as_string) = request.param("id"){
                    if let Some((group, id)) = id_as_string.split_once(":"){
                        match db_sensors_id.lock() {
                            Ok(mut db) => {
                                match db.execute_sql_with_one_return::<for_retrieve::SensorQuery>("SELECT * FROM SensorQuery WHERE id==? AND group_id==?", &[&id, &group]){
                                    Ok(result) => {
                                        serde_json::to_string(&result).unwrap_or_default()
                                    },
                                    Err(_) => String::new(),
                                }
                            },
                            Err(_) => String::new(),
                        }
                    }else{
                        String::new()
                    }
                }else{
                    String::new()
                };

                if result.is_empty(){
                    (StatusCode::BadRequest, "Bad Request - Error".to_string())
                }else{
                    (StatusCode::Ok, result)
                }
            }
        );

        let db_sensors = db.clone();
        server.get(
            "/sensors",
            middleware! {|request|
                println!("Handling /sensors");
                let group = request.query().get("gid");
                let sql_query= match group{
                    Some(it) => format!("SELECT * FROM SensorQuery WHERE group_id = {it}"),
                    None => "SELECT * FROM SensorQuery".to_string(),
                };

                let sensors: Vec<json_structs::SensorInfo> = match db_sensors.lock(){
                    Ok(mut db) => {
                        if let Ok(sensors) = db.execute_sql_with_return::<for_retrieve::SensorQuery>(sql_query.as_str(), &[]){
                            sensors.iter().filter_map(|it|{
                                match it.writable {
                                    false => {
                                        let r#type = if it.bits.parse::<u16>().is_ok(){
                                            "Bool"
                                        }else{
                                            "Int"
                                        }.to_string();

                                        Some(
                                            json_structs::SensorInfo{ id: it.id, group_id: it.group_id, r#type, value: it.value }
                                        )
                                    },
                                    _ => None,
                                }
                            })   .collect()
                        }else{
                            Vec::new()
                        }
                    },
                    Err(_) => Vec::new(),
                };

                match serde_json::to_string(&sensors){
                    Ok(it) => it,
                    Err(error) => format!("Error: {error:?}"),
                }
            },
        );

        server.utilize(router! {
            get "**" => |_req, _res| {
                (StatusCode::NotFound, "404 - Error")
            }
        });

        server.listen("127.0.0.1:6767").unwrap();

        Ok(())
    }
}

mod modbus;
mod pin;
mod transport;

mod backend {
    use std::time::Duration;

    use crate::{modbus, transport};

    pub fn main() {
        match transport::new("/dev/ttyS1", 6, 19200, Duration::from_secs(1)) {
            Ok(port) => match modbus::create(port) {
                Ok(mut mb) => match mb.request_register(1, 0, 32){
                    Ok(result) => println!("Result: {result:?}"),
                    Err(error) => println!("Error: {error}"),
                },
                Err(error) => println!("Error creating port: {error}"),
            },
            Err(error) => {
                println!("Error creating port: {error:?}");
            }
        };
    }
}

fn main() {
    // rest_api::main().unwrap()
    backend::main()
}
