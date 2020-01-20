use std::net::{TcpListener, TcpStream};
use std::thread;
use std::io::{Write, BufReader, BufRead, BufWriter};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use uuid::Uuid;
use serde::{Serialize};
use serde_json::{Value};

static MAP: &str = "";
static VERSION: &str = "0.0.3";

struct Connections {
    map: HashMap<Player, BufWriter<TcpStream>>
}

#[derive(PartialEq, Eq, Hash)]
#[derive(Serialize)]
struct Player {
    remote_address: String,
    remote_port: u16,
    nickname: String,
    id: String,
    current_veh_id: String
}

impl Connections {
    pub fn new() -> Connections{
        Connections { map: HashMap::new()}
    }

    pub fn broadcast(&mut self, msg: String) {
        for socket in &mut self.map {
            socket.1.write(msg.as_bytes()).unwrap();
            socket.1.flush().unwrap();
        }
    }

    pub fn add_player(&mut self, player: Player, writer: BufWriter<TcpStream>) {
        self.map.insert(player, writer);
    }

    pub fn remove_player(&mut self, player: Player) {
        self.map.remove(&player);
    }

    pub fn get_list_of_players(&self) -> Vec<Player> {
        self.map.keys().map(|player| {Player::copy(player)}).collect()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

impl Player {
    pub fn new(remote_address: String,
               remote_port: u16,
               nickname: String,
               id: String,
               current_veh_id: String) -> Player {
        Player {
            remote_address,
            remote_port,
            nickname,
            id,
            current_veh_id
        }
    }

    pub fn copy(other: &Player) -> Player {
        Player {
            remote_address: other.remote_address.clone(),
            remote_port: other.remote_port,
            nickname: other.nickname.clone(),
            id: other.id.clone(),
            current_veh_id: other.current_veh_id.clone()
        }
    }
}

fn main() {

    let connections = Arc::new(RwLock::new(Connections::new()));

    //FOR DEBUGGING PURPOSES
    /*print!("Enter IP address to open TCP on (leave empty for localhost): ");
    std::io::stdout().flush().unwrap();
    let tcp_ip = {
        let mut tcp_ip = String::new();
        if std::io::stdin().read_line( &mut tcp_ip).unwrap()==1 {
            String::from("localhost")
        } else {
            tcp_ip
        }
    };

    print!("Enter port number to open TCP on (leave empty for 30813): ");
    std::io::stdout().flush().unwrap();
    let tcp_port = {
        let mut tcp_port = String::new();
        if std::io::stdin().read_line(&mut tcp_port).unwrap()==1 {
            30813u16
        } else {
            match tcp_port.trim().parse::<u16>() {
                Ok(val) => {
                    val
                }
                Err(_) => {
                    println!("Could not convert to u16, setting to 30813");
                    30813u16
                }
            }
        }
    };*/
    //FOR DEBUGGING PURPOSES

    //match TcpListener::bind(format!("{}:{}", tcp_ip, tcp_port)) { THIS IS DEBUG
    match TcpListener::bind("localhost:30813") {
        Ok(listener) => {
            //println!("Listening on {}:{}", tcp_ip, tcp_port); THIS IS DEBUG
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let cons = connections.clone();
                        thread::spawn(move || {
                            handle(cons, stream);
                        });
                    }
                    Err(_) => {
                        println!("Something went wrong while accepting incoming request!");
                    }
                }
            }
        }
        Err(_) => {
            //println!("Could not open server on {}:{}", tcp_ip, tcp_port); THIS IS DEBUG
        }
    }
}
fn handle(connections: Arc<RwLock<Connections>>, stream: TcpStream) {
    //let addr = stream.local_addr().unwrap();      //TEST
    let (mut reader, mut writer) = stream.try_clone().map(|clone| {(BufReader::new(stream), BufWriter::new(clone))}).unwrap();
    let id = Uuid::new_v4().to_string();
    handshake(&mut writer, id.to_string());
    let player = match get_player(&mut reader, id.clone()) {
        None => {
            return;
        }
        Some(player) => {
            player
        }
    };
    update_players_list_and_send(player, connections.clone(), writer);

    loop {
        let mut s = String::new();
        if reader.read_line(&mut s).unwrap()>0 {
            handle_client_msg(&mut s, connections.clone(), id.clone());
        } else {
            on_close();
        }
    }

    //IF WE WANT TO TEST MORE CODE BELOW IS FOR THAT

    /*let name = format!("{}", count);
    let player = Arc::new(Player::new(addr.ip().to_string(),
                                      addr.port(),
                             format!("{}", count), id,
                             "0".to_string()));
    let cl = connections.clone();
    println!("Done 2");
    test(cl, player, writer);
    println!("Tested");
    loop {
        let mut s = String::new();
        if reader.read_line(&mut s).unwrap()>0 {
            connections.write().unwrap().broadcast(format!("{}:{}", &name, s));
        }
        else {
            println!("{} disconnected", name);
            break;
        }
    }*/

    //END OF TESTING CODE
}

fn handshake(writer: &mut BufWriter<TcpStream>, id: String) {
    writer.write(format!("HOLA{}", id).as_bytes()).unwrap();
    if MAP=="" {
        writer.write(b"MAPS").unwrap();
    } else {
        writer.write(format!("MAPC{}", MAP).as_bytes()).unwrap();
    }
    writer.write(format!("VCHK{}", VERSION).as_bytes()).unwrap();
    writer.flush().unwrap();
}

fn get_player(reader: &mut BufReader<TcpStream>, id: String) -> Option<Player> {
    let player;
    loop {
        let mut s = String::new();
        if reader.read_line(&mut s).unwrap()==0 {return Option::None;}
        if &s[..4]=="USER" {
            let json = serde_json::from_str::<Value>(&s[4..]).unwrap();
            let addr = reader.get_mut().local_addr().unwrap();
            player = Player::new(addr.ip().to_string(),
                                    addr.port(),
                                     json["nickname"].as_str().unwrap().to_string(),
                                    id,
                                    "0".to_string());
            break;
        }
    }
    Option::Some(player)
}

fn update_players_list_and_send(player: Player, connections: Arc<RwLock<Connections>>, writer: BufWriter<TcpStream>) {
    let mut connections = connections.write().unwrap();
    connections.add_player(player, writer);
    let list = connections.get_list_of_players();
    connections.broadcast(format!("PLST{}", serde_json::to_string(&list).unwrap()));
}

fn handle_client_msg(msg: &mut String, connections: Arc<RwLock<Connections>>, id: String) {
    let msg = msg.trim();
    let code = &msg[..4];
    let msg = &msg[4..];

    match code {
        "PING" => {

        }
        "CHAT" => {

        }
        "MAPS" => {

        }
        "QUIT" | "2001" => {

        }
        "U-VI" | "U-VE" | "U-VN" | "U-VP" | "U-VL" | "U-VR" => {

        }
        "U-NV" => {

        }
        "C-VS" => {

        }
        _ => {

        }
    }
}

fn on_close() {

}