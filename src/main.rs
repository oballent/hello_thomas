mod models;
mod facilities;
mod network;

use crate::models::{Cargo, EngineType, Engine, TrainCar, Mission, MissionReport, StationCommand, Location};
use crate::facilities::Station;
use crate::network::RailwayNetwork;

use core::net;
use std::sync::{mpsc, Arc};
use std::thread;

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";

// This program demonstrates the physics of Rust and Networking in the context of a train yard on the Island of Sodor. It models the interactions between trains, engines, cars, cargo, and stations, while also showcasing how ownership and borrowing work in Rust to manage complex state and ensure memory safety. The program also includes a simple representation of a railway network with tracks connecting different stations, allowing for the dispatch and routing of trains across the island. Through this simulation, we can explore how Rust's unique features enable us to build a robust and efficient system that mimics real-world logistics and transportation challenges.


fn main() {
    let mut network = RailwayNetwork::new();



    // Create the Tidmouth radio BEFORE the Tidmouth thread exists
    let (tidmouth_tx, tidmouth_rx) = mpsc::channel();
    let tidmouth_location = Location { x: 100.0, y: 100.0 };
    let tidmouth_name = "Tidmouth".to_string();
    let tidmouth_md = facilities::StationMetadata { name: tidmouth_name.clone(), location: tidmouth_location };
    network.register_station(tidmouth_name.clone(), tidmouth_md.location, tidmouth_tx.clone());

    let (brendam_tx, brendam_rx) = mpsc::channel();
    let brendam_location = Location { x: 100.0, y: 350.0 };
    let brendam_name = "Brendam Docks".to_string();
    let brendam_md = facilities::StationMetadata { name: brendam_name.clone(), location: brendam_location };
    network.register_station(brendam_name.clone(), brendam_md.location, brendam_tx.clone());

    let (knapford_tx, knapford_rx) = mpsc::channel();
    let knapford_location = Location { x: 300.0, y: 100.0 };
    let knapford_name = "Knapford".to_string();
    let knapford_md = facilities::StationMetadata { name: knapford_name.clone(), location: knapford_location };
    network.register_station(knapford_name.clone(), knapford_md.location, knapford_tx.clone());

    let (welsworth_tx, welsworth_rx) = mpsc::channel();
    let welsworth_location = Location { x: 300.0, y: 350.0 };
    let welsworth_name = "Welsworth".to_string();
    let welsworth_md = facilities::StationMetadata { name: welsworth_name.clone(), location: welsworth_location };
    network.register_station(welsworth_name.clone(), welsworth_md.location, welsworth_tx.clone());

    let (maron_tx, maron_rx) = mpsc::channel();
    let maron_location = Location { x: 500.0, y: 100.0 };
    let maron_name = "Maron".to_string();
    let maron_md = facilities::StationMetadata { name: maron_name.clone(), location: maron_location };
    network.register_station(maron_name.clone(), maron_md.location, maron_tx.clone());

    let (vicarstown_tx, vicarstown_rx) = mpsc::channel();
    let vicarstown_location = Location { x: 500.0, y: 350.0 };
    let vicarstown_name = "Vicarstown".to_string();
    let vicarstown_md = facilities::StationMetadata { name: vicarstown_name.clone(), location: vicarstown_location };
    network.register_station(vicarstown_name.clone(), vicarstown_md.location, vicarstown_tx.clone());

    let (peel_godred_tx, peel_godred_rx) = mpsc::channel();
    let peel_godred_location = Location { x: 700.0, y: 100.0 };
    let peel_godred_name = "Peel Godred".to_string();
    let peel_godred_md = facilities::StationMetadata { name: peel_godred_name.clone(), location: peel_godred_location };
    network.register_station(peel_godred_name.clone(), peel_godred_md.location, peel_godred_tx.clone());

    network.add_track(&tidmouth_md.name, &knapford_md.name); //we only need to pass references one way because the track is bidirectional. Once the track is laid, both stations can access it through the network's internal data structures.
    network.add_track(&tidmouth_md.name, &peel_godred_md.name);
    network.add_track(&knapford_md.name, &welsworth_md.name);
    network.add_track(&knapford_md.name, &maron_md.name);
    network.add_track(&welsworth_md.name, &brendam_md.name);
    network.add_track(&welsworth_md.name, &maron_md.name);
    network.add_track(&maron_md.name, &vicarstown_md.name);


    
    let shared_network = Arc::new(network);

    // 1. Instantiate the Stations locally
    let tidmouth = Station::new("Tidmouth", Arc::clone(&shared_network), tidmouth_rx);
    let brendam_docks = Station::new("Brendam Docks", Arc::clone(&shared_network), brendam_rx);
    let knapford = Station::new("Knapford", Arc::clone(&shared_network), knapford_rx);
    let welsworth = Station::new("Welsworth", Arc::clone(&shared_network), welsworth_rx);
    let maron = Station::new("Maron", Arc::clone(&shared_network), maron_rx);
    let vicarstown = Station::new("Vicarstown", Arc::clone(&shared_network), vicarstown_rx);
    let peel_godred = Station::new("Peel Godred", Arc::clone(&shared_network), peel_godred_rx);


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
    tidmouth_incoming_cars.into_iter().for_each(|car| {
        match tidmouth_tx.clone().send(StationCommand::IntakeCar { train_car: car, reply_to: tx_reply.clone() }) {
            Ok(_) => println!("Car successfully intaken by Tidmouth!"),
            Err(e) => println!("Failed to intake car: {:?}", e),
        }
        // We will block and wait for Tidmouth to confirm that it has received the car before we send the next one. This simulates a more realistic process where the station needs to acknowledge receipt of each car before accepting the next one, and it also allows us to see the flow of messages between the main thread and the station more clearly in the console output.
        match rx_reply.recv() {
            Ok(result) => match result {
                Ok(_) => println!("Tidmouth confirmed receipt of the car."),
                Err(e) => println!("Tidmouth reported an error while intaking the car: {:?}", e),
            },
            Err(e) => println!("Failed to receive reply from Tidmouth: {:?}", e),
        }
    });


    tidmouth_incoming_engines.into_iter().for_each(|engine| {
        match tidmouth_tx.clone().send(StationCommand::IntakeEngine { engine, reply_to: tx_reply.clone() }) {
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














    println!("{YELLOW}System Online. Spawning independent customer threads...{RESET}");


    // 3. Spawn Producer 1
    let network_clone_1 = Arc::clone(&shared_network);
    let producer_1_handle = thread::spawn(move || {
        println!("Producer 1: Submitting Mission 1 to the Network...");
        // The Producer threads will create the Mission payloads and send them to the Network. The Network will then process these missions by dispatching trains across the network to fulfill them. After processing each mission, the Network will send a report back to the respective producer thread through the reply channel included in the mission payload, allowing the producers to track the status of their missions and print out the results.
        let (tx_reply1, rx_reply1) = mpsc::channel();
        let mut mission1 = Mission { 
            id: 1, 
            request_id: 1001,
            origin: String::from("Tidmouth"), 
            destination: String::from("Brendam Docks"), 
            required_cars: vec![2, 4], 
            reply_channel: Some(tx_reply1) 
        };
        // Network creates the Conductor in the background!
        network_clone_1.dispatch_train_across_network(mission1); 
        
        // Wait for the final report from the Conductor
        if let Ok(report) = rx_reply1.recv() {
            println!("Producer 1 received report: {:?}", report);
        }
    });

    // 4. Spawn Producer 2
    let network_clone_2 = Arc::clone(&shared_network);
    let producer_2_handle = thread::spawn(move || {
        // Same thing for Producer 2, but with a different mission!
        let (tx_reply2, rx_reply2) = mpsc::channel();
        let mut mission2 = Mission{
            id:2, 
            request_id: 2002,
            origin: String::from("Tidmouth"),
            destination: String::from("Brendam Docks"),
            required_cars: vec![6],
            reply_channel: Some(tx_reply2)
        };

        println!("Producer 2: Submitting Mission 2 to the Network...");
        network_clone_2.dispatch_train_across_network(mission2); 
        
        if let Ok(report) = rx_reply2.recv() {
            println!("Producer 2 received report: {:?}", report);
        }
    });

    // 5. The "smart wait" for the producers to finish. We don't want to just sleep the main thread for an arbitrary amount of time; we want to actually wait for the producer threads to complete their work before we proceed with printing the final station status and shortest route. By calling join() on each producer thread handle, we ensure that the main thread will block until each producer thread has finished executing, which means we'll have received all the mission reports and printed them out before we move on to the next steps in the main thread.
    println!("{YELLOW}Waiting for producer threads to complete...{RESET}");
        producer_1_handle.join().unwrap();
        producer_2_handle.join().unwrap();
    println!("{BOLD}{GREEN}Simulation Complete.{RESET}");













    // //Threads and Wormholes: To demonstrate the physics of Rust and Networking, we will establish a simple radio communication system using Rust's mpsc channels to simulate the dispatch of missions across the network. Each thread will represent a different controle center (or perhaps, a customer) sending out mission updates, while the main thread will listen for these updates and print them out as they are received.

    // //Establish a simple radio communication system using Rust's mpsc channels to simulate the dispatch of missions across the network. Each thread will represent a different controle center (or perhaps, a customer) sending out mission updates, while the main thread will listen for these updates and print them out as they are received.
    // let (tx, rx) = mpsc::channel();

    // //Give a radio to a concurrent thread, Producer 1, to send out a mission update. Since tx is moved into the closure, we need to clone it for each thread that wants to send messages. This allows multiple threads to send messages through the same channel without ownership conflicts.
    // let tx1 = tx.clone();
    // thread::spawn(move || {
    //     // 1. Create a personal radio just for this thread. This will send a MissionReport back to the main thread after the mission is processed so it can print the station status.
    //     let (tx_reply, rx_reply) = mpsc::channel();
        
    //     // 2. put the transmitter inside the Mission payload so the network can send a report back to the main thread after processing the mission. This is a common pattern in Rust for asynchronous communication, where you include a sender in the message payload to allow the recipient to send a response back to the original sender.
    //     let mission1: Mission = Mission { 
    //         id: 1, 
    //         request_id: 1001,
    //         origin: String::from("Tidmouth"), 
    //         destination: String::from("Brendam Docks"), 
    //         required_cars: vec![2, 4], 
    //         reply_channel: Some(tx_reply) 
    //     };

    //     // 3. Send the mission to the Network via the main thread's receiver. The main thread will then process the mission and send a report back to this thread through the tx_reply channel.
    //     tx1.send(mission1).unwrap();
    //     println!("Thread 1 sending mission 1.");

    //     // 4. Block and wait for the Network's report on the mission. Once it receives the report, it can print it out or take further actions based on the success or failure of the mission.
    //     match rx_reply.recv() {
    //         Ok(report) => match report {
    //             MissionReport::Success(message) => println!("{GREEN}Thread 1 received success report: {}{RESET}", message),
    //             MissionReport::Failure(message) => println!("{RED}Thread 1 received failure report: {}{RESET}", message),
    //         },
    //         Err(e) => println!("{RED}Network radio went silent. (Dispatcher dropped.) Error: {}{RESET}", e),
    //     }
    // });

    // // Cloning a radio for a second producer, Producer 2, to send out a different mission update concurrently.
    // let tx2  = tx.clone();
    // thread::spawn(move || {
    //     //1. Do the same thing for Producer 2, create a personal radio and include the sender in the mission payload so the network can send a report back to the main thread after processing the mission.
    //     let (tx_reply, rx_reply) = mpsc::channel();

    //     //2. Place the "radio" (sender) inside the Mission payload so the network can send a report back to the main thread after processing the mission.
    //     let mission2: Mission = Mission{
    //         id:2, 
    //         request_id: 1002,
    //         origin: String::from("Tidmouth"),
    //         destination: String::from("Brendam Docks"), 
    //         required_cars: vec![6],
    //         reply_channel: Some(tx_reply),
    //     };

    //     //3. Send the mission to the Network via the main thread's receiver.
    //     tx2.send(mission2).unwrap();
    //     println!("Thread 2 sending Mission 2.");

    //     //4. Block and wait for the Network's report on the mission. Once it receives the report, it can print it out or take further actions based on the success or failure of the mission.
    //     match rx_reply.recv() {
    //         Ok(report) => match report {
    //             MissionReport::Success(message) => println!("{GREEN}Thread 2 received success report: {}{RESET}", message),
    //             MissionReport::Failure(message) => println!("{RED}Thread 2 received failure report: {}{RESET}", message),
    //         },
    //         Err(e) => println!("{RED}Network radio went silent. (Dispatcher dropped.) Error: {}{RESET}", e),
    //     }
    // });

    // // The Network acts as a single consumer. It listens for incoming mission updates from any producer thread and processes them as they arrive.
    // //The main thread will block on the receiver (rx) until it gets a message and listen.
    // //Every time a mission comes through the radio from any producer thread, the main thread will process the mission by adding it to the network and dispatching a train across the network to fulfill said mission. 
    // for received_mission in rx {
    //     println!{"Main thread received mission updated: {:?}", received_mission};

    //     let received_mission_id = received_mission.id;
    //     network.add_mission(received_mission);//truth be told, a Mission is pretty cheap to clone, but we can also just move it into the network since we won't need to use it in the main thread after this point.

    //     network.dispatch_train_across_network(received_mission_id);
    // }

}








