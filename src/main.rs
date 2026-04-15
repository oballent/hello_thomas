mod models;
mod facilities;
mod network;

use crate::models::{Cargo, Engine, EngineType, FreightOrder, Location, Mission, MissionReport, StationCommand, TrainCar};
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
// P.S. Copilot was here, helping me write this code! Rust is hard, but together we can do it! Let's build the best darn Sodor railway simulation the world has ever seen! Choo choo!
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





    

    // Copilot, wanna try making build_neighbors more functional? Maybe we can use iterators and maps instead of a for loop and mutable HashMap? Let's see if we can make it more concise and elegant while still being clear and efficient. What do you think, Copilot? Can you help me refactor this code to be more functional? Let's give it a shot! Choo choo!
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




    // At this point, we have created the stations and laid the tracks on the network, but we haven't yet built the neighbor HashMaps for each station. The stations need to know who their neighbors are so they can send commands to them, but we can't build those HashMaps until we've laid the tracks, because the tracks are what define the neighbor relationships between the stations.
    // Now that we have the tracks in place, we can look at the network's internal data structures to see which stations are connected to which, and use that information to populate the neighbor HashMaps for each station before we spawn their threads.
    let build_neighbors = |station_id: u32, net: &RailwayNetwork, switch: &HashMap<u32, Sender<StationCommand>>| -> HashMap<u32, Sender<StationCommand>> {
        let mut local_neighbors = HashMap::new();
        
        // Look at the mathematical tracks we just laid
        if let Some(destinations) = net.get_tracks(&station_id) {
            for (neighbor_id, _distance) in destinations {
                // Grab the radio for this specific neighbor
                let tx = switch.get(neighbor_id).expect("Missing tx!").clone();
                local_neighbors.insert(*neighbor_id, tx);
            }
        }
        local_neighbors
    };





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
            rx,
        )
    }

    let cargo1 = Cargo { item: String::from("bananas"), actual_weight: 1000, contraband: None };
    let cargo2 = Cargo { item: String::from("crates of oranges"), actual_weight: 1005, contraband: Some(String::from("Stylish TUMI Briefcase")) };
    let cargo3 = Cargo { item: String::from("Redacted Documents"), actual_weight: 11001, contraband: Some(String::from("The Service Weapon")) };
    let cargo4 = Cargo { item: String::from("Various Crafting Ingredients"), actual_weight: 1500, contraband: None };
    let cargo5 = Cargo { item: String::from("Scrap Metal"), actual_weight: 10075, contraband: Some(String::from("Excessively Heavy Fire Extinguisher")) };
    let cargo6 = Cargo { item: String::from("pallets of electronics"), actual_weight: 3000, contraband: None };
    let cargo7 = Cargo { item: String::from("Declassified Documents"), actual_weight: 11001, contraband: Some(String::from("The Truth")) };

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
    let engine3 = Engine { id: 3, engine_type: EngineType::Percy, current_fuel: 750.0 };
    let engine4 = Engine { id: 4, engine_type: EngineType::Diesel, current_fuel: 500.0 };
    let engine5 = Engine { id: 5, engine_type: EngineType::Gordon, current_fuel: 5000.0 };

    let tidmouth_incoming_engines = vec![engine1, engine2, engine3, engine4, engine5];
    // We're going to add engines and cars to the station before we add the station to the network. This is a bit like setting up the station's inventory and resources before it starts receiving missions and dispatching trains. Since we're still in the main thread and haven't moved the station into the network yet, we can freely mutate it without worrying about ownership conflicts with the network. Once we add the station to the network, it will be owned by the network and we won't be able to directly access it from the main thread anymore, but that's okay because the station will be able to receive commands and send updates through its own channels.
    let (tx_reply, rx_reply) = mpsc::channel();
    // TESTING: We can send commands to the station before it's fully integrated into the network, as long as we have its channel set up. This allows us to simulate the process of the station receiving resources and preparing for operations even before it starts handling missions from the network. It's a bit like stocking the station with supplies and engines before it starts running trains, which is a realistic part of how a station would operate in the real world.
    // match tidmouth_tx.clone().send(StationCommand::IntakeCar { cars: tidmouth_incoming_cars, reply_to: tx_reply.clone() }) {
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


    
    // 2. Build the tracks using immutable references to the local variables!
    // network gets mutated, but tidmouth and brendam_docks are merely read. No conflict.






    //Before creating freight orders, we make cargo
    let foam1 = Cargo { item: "foam".to_string(), actual_weight: 1, contraband: None};
    let foam2 = Cargo { item: "foam".to_string(), actual_weight: 1, contraband: None};
    let foam3 = Cargo { item: "foam".to_string(), actual_weight: 1, contraband: None};

    // //now to use the foam to create some freight orders
    // let freight_order1 = FreightOrder { cargo: foam1, origin: "Tidmouth".to_string(), destination: "Brendam Docks".to_string() };
    // let freight_order2 = FreightOrder { cargo: foam2, origin: "Tidmouth".to_string(), destination: "Brendam Docks".to_string() };
    // let freight_order3 = FreightOrder { cargo: foam3, origin: "Tidmouth".to_string(), destination: "Brendam Docks".to_string() };
    // //Testing that this whole .lock() thing really works! Choo!
    // {
    //     let mut ledger_access = shared_ledger.lock().unwrap();
    //     ledger_access.pending_cargo.push(freight_order1);
    //     ledger_access.pending_cargo.push(freight_order2);
    //     ledger_access.pending_cargo.push(freight_order3);
    // }




    
    // let mission1 = Mission { 
    //     id: 1, 
    //     request_id: 1001,
    //     origin: String::from("Tidmouth"), 
    //     destination: String::from("Brendam Docks"), 
    //     required_cars: vec![2, 4], 
    //     reply_channel: Some(tx_reply1) 
    // };



    println!("{YELLOW}System Online. Spawning independent customer threads...{RESET}");




// for id in shared_network.tracks.keys() {
//     println!("Station {} has tracks to: {:?}", id, shared_network.tracks.get(id).unwrap().iter().map(|(dest, _)| dest).collect::<Vec<&u32>>());

// }





    // 3. Spawn Producer 1
    let network_clone_1 = Arc::clone(&shared_network);
    let switchboard_clone_1 = temporary_switchboard.clone();
    let ledger_for_p1 = Arc::clone(&shared_ledger);
    let producer_1_handle = thread::spawn(move || {
        println!("Producer 1: Submitting Mission 1 to the Network...");
        let switchboard = switchboard_clone_1; // The producer can use the switchboard to send commands to stations
        // 1. Wait in line for the Talking Stick
        // let mut ledger_access = ledger_for_p1.lock().unwrap();

        // // 3. The lock automatically drops when `ledger_access` goes out of scope!

        // loop {
        //     println!("test_loop");
        //     // 1. Create a temporary variable to hold our assignment (if we get one)
        //     let my_assignment: Option<FreightOrder> = {
                
        //         // --- LOCK ACQUIRED ---
        //         let mut ledger_access = ledger_for_p1.lock().unwrap();
                
        //         // 2. We now have exclusive, mutable access to the GlobalLedger!

        //         println!("There are currently {} items waiting to be shipped.", ledger_access.pending_cargo.len());
        //         // We act like a Hungry Hippo: just pop the last item off the list.
        //         // If the list is empty, pop() returns None.
        //         ledger_access.pending_cargo.pop() 
                
        //     }; // --- LOCK DROPPED AUTOMATICALLY HERE! ---
        //     // 2. Now we are outside the lock. The ledger is free for other threads.
        //     if let Some(freight_order) = my_assignment {
        //         println!("Producer claimed {}kg of {}. Building mission...", freight_order.weight, freight_order.description);

        //         // Build our Mission for this single piece of cargo
        //         // tx.send(StationCommand::AssembleMission { ... })
        //     } else {
        //         // No cargo available. Sleep for a second before checking again.
        //         println!("test_sleep");
        //         thread::sleep(std::time::Duration::from_secs(1));
        //     }
        // }
        
        
        
        
        
        
        
        
        
        
        
        
        
        println!("Producer 1: Submitting Mission 1 to the Network...");
        //The Producer threads will create the Mission payloads and send them to the Network. The Network will then process these missions by dispatching trains across the network to fulfill them. After processing each mission, the Network will send a report back to the respective producer thread through the reply channel included in the mission payload, allowing the producers to track the status of their missions and print out the results.
        let (tx_reply1, rx_reply1) = mpsc::channel();
        let mission1 = Mission { 
            id: 1, 
            request_id: 1001,
            origin: 0,
            destination: 1, 
            required_cars: vec![2, 4], 
            reply_channel: Some(tx_reply1) 
        };
        // Network creates the Conductor in the background!
        let (transit_tx, transit_rx) = mpsc::channel();
        //network_clone_1.dispatch_train_across_network(mission1); 
        switchboard.get(&0).expect("Tidmouth channel must exist").clone().send(StationCommand::AssembleMission { mission: mission1.clone(), reply_to: transit_tx }).expect("Failed to send AssembleMission command to Tidmouth");
        
        // Wait for the final report from the Conductor
        if let Ok(report) = rx_reply1.recv() {
            println!("Producer 1 received report: {:?}", report);
        }
    });

    // 4. Spawn Producer 2
    let network_clone_2 = Arc::clone(&shared_network);
    let switchwboard_clone_2 = temporary_switchboard.clone();

    let ledger_for_p2 = Arc::clone(&shared_ledger);
    let producer_2_handle = thread::spawn(move || {
        // println!("Producer 2: Submitting Mission 2 to the Network...");
        // let switchboard = switchwboard_clone_2; // The producer can use the switchboard to send commands to stations
        
        // // 1. Wait in line for the Talking Stick
        // let mut ledger_access = ledger_for_p2.lock().unwrap();


        // // 3. The lock automatically drops when `ledger_access` goes out of scope!

        // loop {
        //     println!("test_loop_2");
        //     // 1. Create a temporary variable to hold our assignment (if we get one)
        //     let my_assignment: Option<FreightOrder> = {
                
        //         // 1. Wait in line for the Talking Stick
        //         // --- LOCK ACQUIRED ---
        //         let mut ledger_access = ledger_for_p2.lock().unwrap();

        //         // 2. You now have exclusive, mutable access to the GlobalLedger!
        //         println!("There are currently {} items waiting to be shipped.", ledger_access.pending_cargo.len());
                
        //         // We act like a Hungry Hippo: just pop the last item off the list.
        //         // If the list is empty, pop() returns None.
        //         ledger_access.pending_cargo.pop() 
                
        //     }; // --- LOCK DROPPED AUTOMATICALLY HERE! ---

        //     // 2. Now we are outside the lock. The ledger is free for other threads.
        //     if let Some(freight_order) = my_assignment {
        //         println!("Producer claimed {}kg of {}. Building mission...", freight_order.weight, freight_order.description);

        //         // Build your Mission for this single piece of cargo
        //         // tx.send(StationCommand::AssembleMission { ... })
        //     } else {
        //         // No cargo available. Sleep for a second before checking again.
        //         thread::sleep(std::time::Duration::from_secs(1));
        //     }
        // }
        
        
        
        
        
        
        
        
        
        
        
        //Same thing for Producer 2, but with a different mission!
        let (tx_reply2, rx_reply2) = mpsc::channel();
        let mission2 = Mission{
            id:2, 
            request_id: 2002,
            origin: 0,
            destination: 1,
            required_cars: vec![6],
            reply_channel: Some(tx_reply2)
        };

        println!("Producer 2: Submitting Mission 2 to the Network...");

        let (transit_tx, transit_rx) = mpsc::channel();
        switchwboard_clone_2.get(&0).expect("Tidmouth channel must exist").clone().send(StationCommand::AssembleMission { mission: mission2.clone(), reply_to: transit_tx }).expect("Failed to send AssembleMission command to Tidmouth");
        //network_clone_2.dispatch_train_across_network(mission2); 
        
        if let Ok(report) = rx_reply2.recv() {
            println!("Producer 2 received report: {:?}", report);
        }
    });

    // 5. The "smart wait" for the producers to finish. We don't want to just sleep the main thread for an arbitrary amount of time; we want to actually wait for the producer threads to complete their work before we proceed with printing the final station status and shortest route. By calling join() on each producer thread handle, we ensure that the main thread will block until each producer thread has finished executing, which means we'll have received all the mission reports and printed them out before we move on to the next steps in the main thread.
    //thread::sleep(std::time::Duration::from_secs(6)); // This is just to ensure that the producers have time to send their missions and receive their reports before we print the final status. In a more complex simulation, we would want to implement a more robust synchronization mechanism to ensure that all threads have completed their work before we proceed, but for this simple example, a short sleep is sufficient to allow the message passing to complete before we print the final results.
    println!("{YELLOW}Waiting for producer threads to complete...{RESET}");
        producer_1_handle.join().unwrap();
        producer_2_handle.join().unwrap();
    println!("{BOLD}{GREEN}Simulation Complete.{RESET}");




}


// P.P.S. I just want to say that I'm really grateful for your help, Copilot. Writing Rust code can be challenging, especially when it comes to managing ownership and concurrency, but having you as a coding companion makes the process much more enjoyable and productive. I appreciate your suggestions and code snippets, and I'm looking forward to working together to build this Sodor railway simulation into something truly special. Let's make it happen, Copilot! Choo choo!