mod models;
mod facilities;
mod network;

use crate::models::{Cargo, EngineType, TrainError, Engine, TrainCar, Train, Mission, MissionReport, RejectedAsset};
use crate::facilities::{Station, Roundhouse, Railyard, Warehouse};
use crate::network::RailwayNetwork;

use std::sync::{WaitTimeoutResult, mpsc};
use std::thread;
use core::net;
use std::{collections::{HashMap, VecDeque}, u32};

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";

// This program demonstrates the physics of Rust and Networking in the context of a train yard on the Island of Sodor. It models the interactions between trains, engines, cars, cargo, and stations, while also showcasing how ownership and borrowing work in Rust to manage complex state and ensure memory safety. The program also includes a simple representation of a railway network with tracks connecting different stations, allowing for the dispatch and routing of trains across the island. Through this simulation, we can explore how Rust's unique features enable us to build a robust and efficient system that mimics real-world logistics and transportation challenges.


fn main() {
    let mut network = RailwayNetwork::new();

    // 1. Instantiate the Stations locally
    let tidmouth = Station::new("Tidmouth");
    let brendam_docks = Station::new("Brendam Docks");

    // 2. Build the tracks using immutable references to the local variables!
    // network gets mutated, but tidmouth and brendam_docks are merely read. No conflict.
    network.add_tracks(&tidmouth, &brendam_docks, 250);
    network.add_tracks(&brendam_docks, &tidmouth, 250); // We can add the reverse route too, since Sodor is not a one-way street!

    // 3. Now that the metadata is extracted, move the Stations into the Network's ownership
    network.add_station(tidmouth);
    network.add_station(brendam_docks);


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
    let engine2 = Engine { id: 2, engine_type: EngineType::Thomas, current_fuel: 2000.0 };
    let engine3 = Engine { id: 3, engine_type: EngineType::Percy, current_fuel: 500.0 };
    let engine4 = Engine { id: 4, engine_type: EngineType::Diesel, current_fuel: 500.0 };
    let engine5 = Engine { id: 5, engine_type: EngineType::Gordon, current_fuel: 5000.0 };


    let origin_name = String::from("Tidmouth");
    if let Some(origin) = network.get_mut_station(&origin_name) {
        for car in tidmouth_incoming_cars {
            origin.receive_car(car)
        }

        //Block a full-fuel Thomas with a half-fuel Thomas to test the find_suitable_engine method. Since the half_fuel (id: 1) Thomas is technically the correct type for the mission, but doesn't have the fuel to complete it, the roundhouse should skip it and select the Thomas with enough fuel to complete the mission instead.
        origin.house_engine(engine4);
        origin.house_engine(engine1);
        origin.house_engine(engine3);
        origin.house_engine(engine2);
        origin.house_engine(engine5);

        origin.print_status();

    } else {
        println!("Error: {} station not found in the network!", origin_name);
        return;
    }


    //Establish a simple radio communication system using Rust's mpsc channels to simulate the dispatch of missions across the network. Each thread will represent a different station or control center sending out mission updates, while the main thread will listen for these updates and print them out as they are received.
    let (tx, rx) = mpsc::channel();

    //Give a radio to a concurrent thread, Producer 1, to send out a mission update. Since tx is moved into the closure, we need to clone it for each thread that wants to send messages. This allows multiple threads to send messages through the same channel without ownership conflicts.
    let tx1 = tx.clone();
    thread::spawn(move || {
        // 1. Create a personal radio just for this thread. This will send a MissionReport back to the main thread after the mission is processed so it can print the station status.
        let (tx_reply, rx_reply) = mpsc::channel();
        
        // 2. put the transmitter inside the Mission payload so the network can send a report back to the main thread after processing the mission. This is a common pattern in Rust for asynchronous communication, where you include a sender in the message payload to allow the recipient to send a response back to the original sender.
        let mission1: Mission = Mission { 
            id: 1, 
            origin: String::from("Tidmouth"), 
            destination: String::from("Brendam Docks"), 
            required_cars: vec![2, 4], 
            reply_channel: Some(tx_reply) 
        };

        // 3. Send the mission to the Network via the main thread's receiver. The main thread will then process the mission and send a report back to this thread through the tx_reply channel.
        tx1.send(mission1).unwrap();
        println!("Thread 1 sending mission 1.");

        // 4. Block and wait for the Network's report on the mission. Once it receives the report, it can print it out or take further actions based on the success or failure of the mission.
        match rx_reply.recv() {
            Ok(report) => match report {
                MissionReport::Success(message) => println!("{GREEN}Thread 1 received success report: {}{RESET}", message),
                MissionReport::Failure(message) => println!("{RED}Thread 1 received failure report: {}{RESET}", message),
            },
            Err(e) => println!("{RED}Network radio went silent. (Dispatcher dropped.) Error: {}{RESET}", e),
        }
    });

    // Cloning a radio for a second producer, Producer 2, to send out a different mission update concurrently.
    let tx2  = tx.clone();
    thread::spawn(move || {
        //1. Do the same thing for Producer 2, create a personal radio and include the sender in the mission payload so the network can send a report back to the main thread after processing the mission.
        let (tx_reply, rx_reply) = mpsc::channel();

        //2. Place the "radio" (sender) inside the Mission payload so the network can send a report back to the main thread after processing the mission.
        let mission2: Mission = Mission{
            id:2, 
            origin: String::from("Tidmouth"),
            destination: String::from("Brendam Docks"), 
            required_cars: vec![6],
            reply_channel: Some(tx_reply),
        };

        //3. Send the mission to the Network via the main thread's receiver.
        tx2.send(mission2).unwrap();
        println!("Thread 2 sending Mission 2.");

        //4. Block and wait for the Network's report on the mission. Once it receives the report, it can print it out or take further actions based on the success or failure of the mission.
        match rx_reply.recv() {
            Ok(report) => match report {
                MissionReport::Success(message) => println!("{GREEN}Thread 2 received success report: {}{RESET}", message),
                MissionReport::Failure(message) => println!("{RED}Thread 2 received failure report: {}{RESET}", message),
            },
            Err(e) => println!("{RED}Network radio went silent. (Dispatcher dropped.) Error: {}{RESET}", e),
        }
    });

    // The Network acts as a single consumer. It listens for incoming mission updates from any producer thread and processes them as they arrive.
    //The main thread will block on the receiver (rx) until it gets a message and listen.
    //Every time a mission comes through the radio from any producer thread, the main thread will process the mission by adding it to the network and dispatching a train across the network to fulfill said mission. 
    for received_mission in rx {
        println!{"Main thread received mission updated: {:?}", received_mission};

        let received_mission_id = received_mission.id;
        let received_mission_origin = received_mission.origin.clone();
        let received_mission_destination = received_mission.destination.clone();
        network.add_mission(received_mission);//truth be told, a Mission is pretty cheap to clone, but we can also just move it into the network since we won't need to use it in the main thread after this point.

        network.dispatch_train_across_network(received_mission_id);

        

        if let Some(origin) = network.get_station(&received_mission_origin){
            origin.print_status();
        }
        if let Some(destination) = network.get_station(&received_mission_destination){
            destination.print_status();
        }


    }

}








