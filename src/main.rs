use std::net::{TcpListener, TcpStream};
use std::thread;
use std::io::{Write, BufReader, BufRead, BufWriter};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use uuid::Uuid;
use serde::Serialize;
use serde_json::Value;

const VERSION: &str = "0.0.3";

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
    #[serde(rename="currentVehID")]
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

    pub fn broadcast_to_everyone_else(&mut self, msg: String, except: &Player) {
        for socket in &mut self.map {
            if !Player::eq(socket.0, except) {
                socket.1.write(msg.as_bytes()).unwrap();
                socket.1.flush().unwrap();
            }
        }
    }

    pub fn add_player(&mut self, player: Player, writer: BufWriter<TcpStream>) {
        self.map.insert(player, writer);
    }

    pub fn remove_player(&mut self, player: &Player) {
        self.map.remove(player);
    }

    pub fn get_list_of_players(&self) -> Vec<&Player> {
        let mut res = vec![];
        for pair in &self.map {
            res.push(pair.0);
        }
        res
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn send_private(&mut self, msg: String, to: &Player) -> Result<(), &str> {
        let stream = &mut self.map.get_mut(to);
        let stream = match stream {
            Some(stream) => {
                stream
            }
            None => {
                return Err("No such player found in player list");
            }
        };
        match stream.write(msg.as_bytes()) {
            Ok(_) => {
                Ok(())
            }
            Err(_) => {
                Err("Error while writing to stream")
            }
        }
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

    pub fn eq(this: &Player, other: &Player) -> bool {
        this.id==other.id
    }
}

fn main() {
    let map = Arc::new(RwLock::new(String::new()));
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
            println!("Listening on localhost:30813");
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        println!("Got connection!");
                        let cons = connections.clone();
                        let connections = connections.read().unwrap();
                        if connections.len() < 8 {
                            let map_cl = map.clone();
                            thread::spawn(move || {
                                handle(cons, stream, map_cl);
                            });
                        } else {
                            println!("Denied: Server full (max 8 players)");
                        }
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
fn handle(connections: Arc<RwLock<Connections>>, stream: TcpStream, map: Arc<RwLock<String>>) {
    //let addr = stream.local_addr().unwrap();      //TEST
    let (mut reader, writer) = stream.try_clone().map(|clone| {(BufReader::new(stream), BufWriter::new(clone))}).unwrap();
    let id = Uuid::new_v4().to_string();

    let mut player = match handshake(writer, &mut reader, &connections, id, &map) {
        Ok(player) => {
            println!("Handshake successful");
            player
        }
        Err(string) => {
            println!("An error occurred!\n{}", string);
            return;
        }
    };

    loop {
        let mut s = String::new();
        let mut online = true;
        if reader.read_line(&mut s).unwrap()>0 {
            online = handle_client_msg(s, &connections, &mut player, &map);
            if !online {break;}
        } else {
            on_close(&connections, &mut player);
            break;
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

fn handshake<'a>(mut writer: BufWriter<TcpStream>, reader: &'a mut BufReader<TcpStream>, connections: &'a Arc<RwLock<Connections>>, id: String, map: &Arc<RwLock<String>>) -> Result<Player, &'a str> {
    writer.write(format!("HOLA{}", id).as_bytes()).unwrap();
    if *map.read().unwrap() == "" {
        writer.write(b"MAPS").unwrap();
    } else {
        writer.write(format!("MAPC{}", *map.read().unwrap()).as_bytes()).unwrap();
    }
    writer.write(format!("VCHK{}", VERSION).as_bytes()).unwrap();
    writer.flush().unwrap();

    let player = match get_player(reader, id) {
        Ok(player) => {
            player
        }
        Err(msg) => {
            return Err(msg);
        }
    };

    match update_players_list_and_send(&player, connections, Option::Some(writer), true) {
        Ok(_) => {
            Ok(player)
        }
        Err(_) => {
            Err("Could not update players list for some reason")
        }
    }
}

fn get_player(reader: &mut BufReader<TcpStream>, id: String) -> Result<Player, &str> {
    let mut count = 0u8;
    while count < 10 {
        let mut s = String::new();
        if reader.read_line(&mut s).unwrap()==0 {return Err("Client disconnected during handshake");}
        if &s[..4]=="USER" {
            let json = serde_json::from_str::<Value>(&s[4..]).unwrap();
            let addr = reader.get_mut().local_addr().unwrap();
            return Ok(Player::new(addr.ip().to_string(),
                                    addr.port(),
                                     json["nickname"].as_str().unwrap().to_string(),
                                    id,
                                    "0".to_string()));
        }
        count +=1;
    }
    Err("Client did not give information about themselves (\"USER\" code was not received)")
}

fn update_players_list_and_send<'a>(player: &Player, connections: &'a Arc<RwLock<Connections>>, writer: Option<BufWriter<TcpStream>>, op: bool) -> Result<(), &'a str> {
    let mut connections = connections.write().unwrap();
    if op {connections.add_player(Player::copy(player), writer.unwrap());}
    else {connections.remove_player(player);}
    let list = connections.get_list_of_players();
    let list = serde_json::to_string(&list).expect("Error parsing json list");
    connections.broadcast(format!("PLST{}", list));
    Ok(())
}

fn handle_client_msg(msg: String, connections: &Arc<RwLock<Connections>>, player: &mut Player, map: &Arc<RwLock<String>>) -> bool {
    let msg = msg.trim();
    let code = &msg[..4];
    let msg = msg[4..].to_string();

    if code == "QUIT" || code == "2001" {on_close(connections, player); return false;}

    let mut connections = connections.write().unwrap();
    match code {
        "PING" => {
            connections.send_private("PONG".to_string(), player);
        }
        "CHAT" => {
            connections.broadcast(msg);
        }
        "MAPS" => {
            let mut map = map.write().unwrap();
            *map = msg;
        }
        "U-VI" | "U-VE" | "U-VN" | "U-VP" | "U-VL" | "U-VR" => {
            connections.broadcast_to_everyone_else(msg, player);
        }
        "U-VC" => {
            connections.broadcast(msg);
        }
        "U-NV" => {
            println!("U-NV:\n{}", msg);
            //TODO new id???
        }
        "C-VS" => {
            println!("C-VS:\n{}", msg);
            if player.current_veh_id != msg {
                player.current_veh_id = msg;
            }
        }
        _ => {
            println!("Unknown request from {}:{} (nickname: {}):\n{}", player.remote_address,
                                                        player.remote_port,
                                                        player.nickname,
                                                        msg);
        }
    }
    true
}

fn on_close(connections: &Arc<RwLock<Connections>>, player: &mut Player) {
    println!("Player {} disconnected", player.nickname);
    update_players_list_and_send(player, connections, Option::None, false);
}
