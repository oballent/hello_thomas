mod models;
mod facilities;
mod network;

use crate::models::{Producer, Cargo, Engine, EngineType, FreightOrder, Location, Mission, MissionReport, StationCommand, TrainCar};
use crate::facilities::Station;
use crate::network::{RailwayNetwork, GlobalLedger};

use core::net;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::collections::{HashMap};

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub stations: Vec<StationConfig>,
    pub tracks: Vec<TrackConfig>,
}

#[derive(Deserialize, Debug)]
pub struct StationConfig {
    pub id: u32,
    pub name: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Deserialize, Debug)]
pub struct TrackConfig {
    pub origin: u32,
    pub destination: u32,
}

// This program demonstrates the physics of Rust and Networking in the context of a Railway Network on the Island of Sodor.
// It has evolved from a simple monolithic architecture to a more complex asynchronous and distributed system.
// Each station operates independently and communicates with its neighbors through message passing.
// The main thread serves as the central hub for initializing the network and spawning producer threads that create missions for the stations to fulfill.
// Each station runs in its own thread, managing its internal state and resources while responding to commands from the producers and coordinating with neighboring stations to dispatch trains across the network.
// The use of channels for communication and Arc/Mutex for shared state allows us to maintain thread safety while enabling a dynamic and interactive simulation of a railway system.

// I AM The Fat Controller - or, at least, I would be if this were built around a Monolithic Architecture - but it's not!
// Asynchronous and Distributed systems are the name of the game! All aboard for a Rustacean adventure on the Island of Sodor! Choo choo!
// P.S. Copilot, Gemini, and ChatGPT were here, helping me write this code! Rust is hard, but together we can do it! Let's build the best darn Sodor railway simulation the world has ever seen! Choo choo!
// Ahem, let's get this show on the rails!

fn main() {

    // 1. Read the raw text from the file
    let file_content = std::fs::read_to_string("sodor.json").expect("Failed to read sodor.json");
    
    // 2. SERDE MAGIC: Convert the JSON text directly into our Rust Structs!
    let config: Config = serde_json::from_str(&file_content).expect("Failed to parse JSON");

    println!("{GREEN}Loaded {} stations and {} tracks from config.{RESET}", config.stations.len(), config.tracks.len());

    let mut network = RailwayNetwork::new();
    let mut temporary_switchboard: HashMap<u32, Sender<StationCommand>> = HashMap::new();

    // NEW: A temporary holding pen for the receivers!
    let mut temporary_receivers: HashMap<u32, Receiver<StationCommand>> = HashMap::new();


    for station in &config.stations {
        let (tx, rx) = mpsc::channel();
        temporary_switchboard.insert(station.id, tx.clone());
        temporary_receivers.insert(station.id, rx);
        
        let loc = Location { x: station.x, y: station.y };
        network.register_station(station.id, loc);
    }

    for track in &config.tracks {
        network.add_track(track.origin, track.destination);
    }

    // 1. Create the raw data
    let ledger = GlobalLedger::new();
    // 2. Put it in the Mutex Vault (The Talking Stick)
    let locked_ledger = Mutex::new(ledger);
    // 3. Put the Vault in an Arc so multiple threads can find it
    let shared_ledger = Arc::new(locked_ledger);




    // Copilot, wanna try making build_neighbors more functional? Maybe we can use iterators and maps instead of a for loop and mutable HashMap? Let's give it a shot! Choo choo!
    // NEW AND IMPROVED: A more functional approach to building the neighbors HashMap for each station! Instead of using a mutable HashMap and a for loop, we can use iterators and the filter_map method to create the neighbors HashMap in a more concise and elegant way. This approach is more in line with Rust's functional programming style and can be easier to read once you're familiar with the iterator methods. Let's see how it looks!

    let build_neighbors = |station_id: u32, net: &RailwayNetwork, switch: &HashMap<u32, Sender<StationCommand>>| -> HashMap<u32, Sender<StationCommand>> {
        net.get_tracks(&station_id)
            .into_iter()     // Turn the Option into an Iterator (yields 0 or 1 item)
            .flatten()       // Flatten the inner Vec into a stream of (dest_id, distance) tuples
            .map(|(dest_id, _distance)| {
                let tx = switch.get(dest_id).expect("Missing tx!").clone();
                (*dest_id, tx)
            })
            .collect()       // Automatically gather the (K, V) tuples into a HashMap!
    };

    // The above function works as follows:
    // 1. We call net.get_tracks(&station_id) to get the Option<&Vec<(u32, f64)>> of tracks from the station. This will yield Some(vec) if there are tracks, or None if there are no tracks.
    // 2. We use into_iter() to turn the Option into an Iterator. If it's Some(vec), we get an iterator over that vec. If it's None, we get an empty iterator.
    // 3. We use flatten() to take the inner Vec<(u32, f64)> and turn it into a stream of (dest_id, distance) tuples. If there were no tracks, this will just be an empty stream.
    // 4. We use map() to transform each (dest_id, distance) tuple into a (dest_id, tx) tuple, where tx is the Sender<StationCommand> for that destination station. We look up the tx in the switchboard HashMap using switch.get(dest_id), and we clone it to get a new Sender that we can store in our neighbors HashMap.
    // 5. Finally, we use collect() to gather all the (dest_id, tx) tuples into a HashMap<u32, Sender<StationCommand>>, which is the type we want for our neighbors. If there were no tracks, this will just be an empty HashMap.
    // Nice.




    // // At this point, we have created the stations and laid the tracks on the network, but we haven't yet built the neighbor HashMaps for each station. The stations need to know who their neighbors are so they can send commands to them, but we can't build those HashMaps until we've laid the tracks, because the tracks are what define the neighbor relationships between the stations.
    // // Now that we have the tracks in place, we can look at the network's internal data structures to see which stations are connected to which, and use that information to populate the neighbor HashMaps for each station before we spawn their threads.
    // let build_neighbors = |station_id: u32, net: &RailwayNetwork, switch: &HashMap<u32, Sender<StationCommand>>| -> HashMap<u32, Sender<StationCommand>> {
    //     let mut local_neighbors = HashMap::new();
        
    //     // Look at the mathematical tracks we just laid
    //     if let Some(destinations) = net.get_tracks(&station_id) {
    //         for (neighbor_id, _distance) in destinations {
    //             // Grab the radio for this specific neighbor
    //             let tx = switch.get(neighbor_id).expect("Missing tx!").clone();
    //             local_neighbors.insert(*neighbor_id, tx);
    //         }
    //     }
    //     local_neighbors
    // };





    let shared_network = Arc::new(network);


    for station in &config.stations {
        let neighbors = build_neighbors(station.id, &shared_network, &temporary_switchboard);
        println!("Station {} has neighbors: {:?}", station.name, neighbors.keys().collect::<Vec<&u32>>());

        let tx = temporary_switchboard.get(&station.id).expect("Missing tx!").clone();
        let rx = temporary_receivers.remove(&station.id).expect("Missing rx!");

        
        Station::new(
            station.id, 
            &station.name, 
            neighbors, 
            tx, 
            Arc::clone(&shared_network), 
            Arc::clone(&shared_ledger),
            rx,
        )
    }

    let cargo1 = Cargo { id:0, item: String::from("bananas"), actual_weight: 1000, contraband: None };
    let cargo2 = Cargo { id:1, item: String::from("crates of oranges"), actual_weight: 1005, contraband: Some(String::from("Stylish TUMI Briefcase")) };
    let cargo3 = Cargo { id:2, item: String::from("Redacted Documents"), actual_weight: 11001, contraband: Some(String::from("The Service Weapon")) };
    let cargo4 = Cargo { id:3, item: String::from("Various Crafting Ingredients"), actual_weight: 1500, contraband: None };
    let cargo5 = Cargo { id:4, item: String::from("Scrap Metal"), actual_weight: 10075, contraband: Some(String::from("Excessively Heavy Fire Extinguisher")) };
    let cargo6 = Cargo { id:5, item: String::from("pallets of electronics"), actual_weight: 3000, contraband: None };
    let cargo7 = Cargo { id:6, item: String::from("Declassified Documents"), actual_weight: 11001, contraband: Some(String::from("The Truth")) };

    let carriage = TrainCar { id:1, cargo: Some(cargo2), passenger: Some(String::from("Lemon:"))};
    let dining_car = TrainCar { id:2, cargo: Some(cargo1), passenger: Some(String::from("Ladybug"))};
    let boxcar1 = TrainCar { id:3, cargo: Some(cargo5), passenger: Some(String::from("Blazkowicz")),};
    let boxcar2 = TrainCar { id:4, cargo: Some(cargo6), passenger: Some(String::from("Tangerine")),};
    let boxcar3 = TrainCar { id:5, cargo: Some(cargo3), passenger: Some(String::from("Faden")),}; 
    let boxcar4 = TrainCar { id:5, cargo: Some(cargo7), passenger: Some(String::from("Mathison")),};
    let caboose = TrainCar { id:6, cargo: Some(cargo4), passenger: Some(String::from("Artyom"))};

    let tidmouth_incoming_cars = vec![carriage, dining_car, boxcar1, boxcar2, boxcar3, boxcar4, caboose];


    let engine1 = Engine { id: 1, engine_type: EngineType::Thomas, current_fuel: 1000.0 };
    let engine2 = Engine { id: 2, engine_type: EngineType::Thomas, current_fuel: 3000.0 };
    let engine3 = Engine { id: 3, engine_type: EngineType::Percy, current_fuel: 1000.0 };
    let engine4 = Engine { id: 4, engine_type: EngineType::Diesel, current_fuel: 750.0 };
    let engine5 = Engine { id: 5, engine_type: EngineType::Gordon, current_fuel: 5000.0 };

    let emergency_gordon_1 = Engine { id: 6, engine_type: EngineType::Gordon, current_fuel: 5000.0 };
    let emergency_gordon_2 = Engine { id: 7, engine_type: EngineType::Gordon, current_fuel: 5000.0 };
    let emergency_gordon_3 = Engine { id: 8, engine_type: EngineType::Gordon, current_fuel: 5000.0 };
    let emergency_gordon_4 = Engine { id: 9, engine_type: EngineType::Gordon, current_fuel: 5000.0 };
    let emergency_gordon_5 = Engine { id: 10, engine_type: EngineType::Gordon, current_fuel: 5000.0 };
    let emergency_gordon_6 = Engine { id: 11, engine_type: EngineType::Gordon, current_fuel: 5000.0 };
    let emergency_gordon_7 = Engine { id: 12, engine_type: EngineType::Gordon, current_fuel: 5000.0 };



    let tidmouth_incoming_engines = vec![engine1, engine2, engine3, engine4, engine5, emergency_gordon_7];
    // We're going to add engines and cars to the station before we add the station to the network. This is a bit like setting up the station's inventory and resources before it starts receiving missions and dispatching trains. Since we're still in the main thread and haven't moved the station into the network yet, we can freely mutate it without worrying about ownership conflicts with the network. Once we add the station to the network, it will be owned by the network and we won't be able to directly access it from the main thread anymore, but that's okay because the station will be able to receive commands and send updates through its own channels.
    let (tx_reply, rx_reply) = mpsc::channel();
    // TESTING: We can send commands to the station before it's fully integrated into the network, as long as we have its channel set up. This allows us to simulate the process of the station receiving resources and preparing for operations even before it starts handling missions from the network. It's a bit like stocking the station with supplies and engines before it starts running trains, which is a realistic part of how a station would operate in the real world.
    // match tidmouth_tx.clone().send(StationCommand::IntakeCar { cars: tidmouth_incoming_cars, reply_to: tx_reply.clone() }) {
    
    let brendam_docks_incoming_engines = vec![emergency_gordon_1];

    let knapford_incoming_engines = vec![emergency_gordon_2];

    let welsworth_incoming_engines = vec![emergency_gordon_3];

    let peel_godred_incoming_engines = vec![emergency_gordon_4];
    
    let vicarstown_incoming_engines = vec![emergency_gordon_5];

    let maron_incoming_engines = vec![emergency_gordon_6];
    
    
    
    
    //WE ARE GOING TO COMMENT OUT OUR TRAINCAR INITIALIZATION, in order to test the functionality of request_empty_cars and the station's ability to respond to that command by creating empty cars on demand. This will allow us to verify that the station can handle requests for resources and manage its inventory of train cars correctly, which is an important part of its functionality in fulfilling missions and dispatching trains across the network. By testing this command, we can ensure that the station is properly integrated with the network and can respond to commands from producers in a dynamic and flexible way. Nice, Copilot! Let's see how the station handles the request for empty cars and make sure it can create them on demand when needed! Choo choo!
    
    match temporary_switchboard.get(&0).expect("Tidmouth channel must exist").clone().send(StationCommand::IntakeCar { cars: tidmouth_incoming_cars, reply_to: tx_reply.clone() }) {
        Ok(_) => println!("Car successfully intaken by Tidmouth!"),
        Err(e) => println!("Failed to intake car: {:?}", e),
    }
    //We will block and wait for Tidmouth to confirm that it has received the car before we send the next one. This simulates a more realistic process where the station needs to acknowledge receipt of each car before accepting the next one, and it also allows us to see the flow of messages between the main thread and the station more clearly in the console output.
    match rx_reply.recv() {
        Ok(result) => match result {
            Ok(_) => println!("Tidmouth confirmed receipt of the car."),
            Err(e) => println!("Tidmouth reported an error while intaking the car: {:?}", e),
        },
        Err(e) => println!("Failed to receive reply from Tidmouth: {:?}", e),
    };









    // TESTING. We can do the same thing with engines, sending them one at a time and waiting for confirmation from Tidmouth before sending the next one. This allows us to simulate the process of the station receiving and processing each engine individually, which is more realistic and also helps us see the message flow more clearly in the console output. It's a bit like how a station would need to inspect and prepare each engine before it can be added to the roundhouse and used for operations.
    tidmouth_incoming_engines.into_iter().for_each(|engine| {
        match temporary_switchboard.get(&0).expect("Tidmouth channel must exist").clone().send(StationCommand::IntakeEngine { engine, reply_to: tx_reply.clone() }) {
            Ok(_) => println!("Engine successfully intaken by Tidmouth!"),
            Err(e) => println!("Failed to intake engine: {:?}", e),
        }
        match rx_reply.recv() {
            Ok(result) => match result {
                Ok(_) => println!("Tidmouth confirmed receipt of the engine."),
                Err(e) => println!("Tidmouth reported an error while intaking the engine: {:?}", e),
            },
            Err(e) => println!("Failed to receive reply from Tidmouth: {:?}", e),
        }
    });

    brendam_docks_incoming_engines.into_iter().for_each(|engine| {
        match temporary_switchboard.get(&1).expect("Brendam Docks channel must exist").clone().send(StationCommand::IntakeEngine { engine, reply_to: tx_reply.clone() }) {
            Ok(_) => println!("Engine successfully intaken by Brendam Docks!"),
            Err(e) => println!("Failed to intake engine: {:?}", e),
        }
        match rx_reply.recv() {
            Ok(result) => match result {
                Ok(_) => println!("Brendam Docks confirmed receipt of the engine."),
                Err(e) => println!("Brendam Docks reported an error while intaking the engine: {:?}", e),
            },
            Err(e) => println!("Failed to receive reply from Brendam Docks: {:?}", e),
        }
    });

    vicarstown_incoming_engines.into_iter().for_each(|engine| {
        match temporary_switchboard.get(&4).expect("Vicarstown channel must exist").clone().send(StationCommand::IntakeEngine { engine, reply_to: tx_reply.clone() }) {
            Ok(_) => println!("Engine successfully intaken by Vicarstown!"),
            Err(e) => println!("Failed to intake engine: {:?}", e),
        }
        match rx_reply.recv() {
            Ok(result) => match result {
                Ok(_) => println!("Vicarstown confirmed receipt of the engine."),
                Err(e) => println!("Vicarstown reported an error while intaking the engine: {:?}", e),
            },
            Err(e) => println!("Failed to receive reply from Vicarstown: {:?}", e),
        }
    });

    maron_incoming_engines.into_iter().for_each(|engine| {
        match temporary_switchboard.get(&5).expect("Maron channel must exist").clone().send(StationCommand::IntakeEngine { engine, reply_to: tx_reply.clone() }) {
            Ok(_) => println!("Engine successfully intaken by Maron!"),
            Err(e) => println!("Failed to intake engine: {:?}", e),
        }
        match rx_reply.recv() {
            Ok(result) => match result {
                Ok(_) => println!("Maron confirmed receipt of the engine."),
                Err(e) => println!("Maron reported an error while intaking the engine: {:?}", e),
            },
            Err(e) => println!("Failed to receive reply from Maron: {:?}", e),
        }
    });
    
    welsworth_incoming_engines.into_iter().for_each(|engine| {
        match temporary_switchboard.get(&3).expect("Welsworth channel must exist").clone().send(StationCommand::IntakeEngine { engine, reply_to: tx_reply.clone() }) {
            Ok(_) => println!("Engine successfully intaken by Welsworth!"),
            Err(e) => println!("Failed to intake engine: {:?}", e),
        }
        match rx_reply.recv() {
            Ok(result) => match result {
                Ok(_) => println!("Welsworth confirmed receipt of the engine."),
                Err(e) => println!("Welsworth reported an error while intaking the engine: {:?}", e),
            },
            Err(e) => println!("Failed to receive reply from Welsworth: {:?}", e),
        }
    });

    peel_godred_incoming_engines.into_iter().for_each(|engine| {
        match temporary_switchboard.get(&2).expect("Peel Godred channel must exist").clone().send(StationCommand::IntakeEngine { engine, reply_to: tx_reply.clone() }) {
            Ok(_) => println!("Engine successfully intaken by Peel Godred!"),
            Err(e) => println!("Failed to intake engine: {:?}", e),
        }
        match rx_reply.recv() {
            Ok(result) => match result {
                Ok(_) => println!("Peel Godred confirmed receipt of the engine."),
                Err(e) => println!("Peel Godred reported an error while intaking the engine: {:?}", e),
            },
            Err(e) => println!("Failed to receive reply from Peel Godred: {:?}", e),
        }
    });

    knapford_incoming_engines.into_iter().for_each(|engine| {
        match temporary_switchboard.get(&6).expect("Knapford channel must exist").clone().send(StationCommand::IntakeEngine { engine, reply_to: tx_reply.clone() }) {
            Ok(_) => println!("Engine successfully intaken by Knapford!"),
            Err(e) => println!("Failed to intake engine: {:?}", e),
        }
        match rx_reply.recv() {
            Ok(result) => match result {
                Ok(_) => println!("Knapford confirmed receipt of the engine."),
                Err(e) => println!("Knapford reported an error while intaking the engine: {:?}", e),
            },
            Err(e) => println!("Failed to receive reply from Knapford: {:?}", e),
        }
    });


    
    // 2. Build the tracks using immutable references to the local variables!
    // network gets mutated, but tidmouth and brendam_docks are merely read. No conflict.



    //Before creating freight orders, we make cargo
    let foam1 = Cargo { id: 7, item: "foam".to_string(), actual_weight: 1, contraband: None};
    let foam2 = Cargo { id: 8, item: "foam".to_string(), actual_weight: 1, contraband: None};
    let foam3 = Cargo { id: 9, item: "foam".to_string(), actual_weight: 1, contraband: None};

    let tidmouth_incoming_cargo = vec![foam1, foam2, foam3];

    match temporary_switchboard.get(&0).expect("Tidmouth channel must exist").clone().send(StationCommand::IntakeCargo {
        cargo: tidmouth_incoming_cargo, 
        reply_to: tx_reply.clone() 
    }) {
        Ok(_) => println!("Cargo successfully intaken by Tidmouth!"),
        Err(e) => println!("Failed to intake cargo: {:?}", e),
    }






    //commenting out just to see if intake car pushes to global ledger correctly without us having to wait for the station to confirm receipt of the car. This will allow us to verify that the station is properly updating the global ledger with new cargo as it intakes it, which is an important part of how the station manages its inventory and fulfills missions. By testing this functionality, we can ensure that the station is correctly integrated with the global ledger and can keep track of the cargo it has on hand, which will be crucial for fulfilling freight orders and dispatching trains across the network. Let's see if the cargo IDs show up in the global ledger after we send the IntakeCargo command! Choo choo!
    // //now to use the foam to create some freight orders
    // let freight_order1 = FreightOrder { id: 1, cargo_ids: vec![7], origin: 0, destination: 1 , ttl: 5};
    // let freight_order2 = FreightOrder { id: 2, cargo_ids: vec![8], origin: 0, destination: 1 , ttl: 5};
    // let freight_order3 = FreightOrder { id: 3, cargo_ids: vec![9], origin: 0, destination: 1 , ttl: 5};
    // //Testing that this whole .lock() thing really works! Choo!
    // {
    //     let mut ledger_access = shared_ledger.lock().unwrap();
    //     ledger_access.pending_cargo.push(freight_order1);
    //     ledger_access.pending_cargo.push(freight_order2);
    //     ledger_access.pending_cargo.push(freight_order3);
    // }





    println!("{YELLOW}System Online. Spawning independent customer threads...{RESET}");




    // // 3. Spawn Producer 1
    // let network_clone_1 = Arc::clone(&shared_network);
    // let switchboard_clone_1 = temporary_switchboard.clone();
    // let ledger_for_p1 = Arc::clone(&shared_ledger);

    let producer_1 = Producer::new(1, Arc::clone(&shared_ledger), temporary_switchboard.clone());
    let producer_1_handle = producer_1.start();


    // // 4. Spawn Producer 2
    // let network_clone_2 = Arc::clone(&shared_network);
    // let switchboard_clone_2 = temporary_switchboard.clone();

    let producer_2 = Producer::new(2, Arc::clone(&shared_ledger), temporary_switchboard.clone());
    let producer_2_handle = producer_2.start();
    
    // 5. The "smart wait" for the producers to finish. We don't want to just sleep the main thread for an arbitrary amount of time; we want to actually wait for the producer threads to complete their work before we proceed with printing the final station status and shortest route. By calling join() on each producer thread handle, we ensure that the main thread will block until each producer thread has finished executing, which means we'll have received all the mission reports and printed them out before we move on to the next steps in the main thread.
    //thread::sleep(std::time::Duration::from_secs(6)); // This is just to ensure that the producers have time to send their missions and receive their reports before we print the final status. In a more complex simulation, we would want to implement a more robust synchronization mechanism to ensure that all threads have completed their work before we proceed, but for this simple example, a short sleep is sufficient to allow the message passing to complete before we print the final results.
    println!("{YELLOW}Waiting for producer threads to complete...{RESET}");
        producer_1_handle.join().unwrap();//this is like a gate that ensures the main thread waits for producer 1
        producer_2_handle.join().unwrap();//this as well, but for gate 2 and producer 2.
    println!("{BOLD}{GREEN}Simulation Complete.{RESET}");




}


// P.P.S. I just want to say that I'm really grateful for your help, Copilot. Writing Rust code can be challenging, especially when it comes to managing ownership and concurrency, but having you as a coding companion makes the process much more enjoyable and productive. I appreciate your suggestions and code snippets, and I'm looking forward to working together to build this Sodor railway simulation into something truly special. Let's make it happen, Copilot! Choo choo!