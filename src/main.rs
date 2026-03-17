mod models;
mod facilities;
mod network;

use crate::models::{Cargo, EngineType, TrainError, Engine, TrainCar, Train, Mission, RejectedAsset};
use crate::facilities::{Station, Roundhouse, Railyard,};
use crate::network::RailwayNetwork;

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
    let cargo7 = Cargo { item: String::from("Redacted Documents"), actual_weight: 11001, contraband: Some(String::from("The Service Weapon")) };

    let carriage = TrainCar { id:1, cargo: Some(cargo2), passenger: Some(String::from("Lemon:"))};
    let dining_car = TrainCar { id:2, cargo: Some(cargo1), passenger: Some(String::from("Ladybug"))};
    let boxcar1 = TrainCar { id:3, cargo: Some(cargo5), passenger: Some(String::from("Blazkowicz")),};
    let boxcar2 = TrainCar { id:4, cargo: Some(cargo6), passenger: Some(String::from("Tangerine")),};
    let boxcar3 = TrainCar { id:5, cargo: Some(cargo3), passenger: Some(String::from("Faden")),}; 
    let boxcar4 = TrainCar { id:5, cargo: Some(cargo7), passenger: Some(String::from("Faden")),};
    let caboose = TrainCar { id:6, cargo: Some(cargo4), passenger: Some(String::from("Artyom"))};

    let tidmouth_incoming_cars = vec![carriage, dining_car, boxcar1, boxcar2, boxcar3, boxcar4, caboose];


    let engine4 = Engine { id: 1, engine_type: EngineType::Thomas, current_fuel: 1000.0 };
    let engine2 = Engine { id: 2, engine_type: EngineType::Thomas, current_fuel: 2000.0 };
    let engine3 = Engine { id: 3, engine_type: EngineType::Percy, current_fuel: 500.0 };
    let engine1 = Engine { id: 4, engine_type: EngineType::Diesel, current_fuel: 500.0 };
    let engine5 = Engine { id: 5, engine_type: EngineType::Gordon, current_fuel: 5000.0 };


    let origin_name = String::from("Tidmouth");
    if let Some(origin) = network.get_mut_station(&origin_name) {
        for car in tidmouth_incoming_cars {
            origin.receive_car(car)
        }

        //Switched it up to intentionally block a full-fuel Thomas with a half-fuel Thomas to test the find_suitable_engine method. Since the half_fuel Thomas is technically the correct type for the mission, but doesn't have the fuel to complete it, the roundhouse should skip it and select the Thomas with enough fuel to complete the mission instead.
        origin.house_engine(engine1);
        origin.house_engine(engine4);
        origin.house_engine(engine3);
        origin.house_engine(engine2);
        origin.house_engine(engine5);

        origin.print_status();

    } else {
        println!("Error: {} station not found in the network!", origin_name);
    }


    let mission1: Mission = Mission { id: 1, origin: String::from("Tidmouth"), destination: String::from("Brendam Docks"), required_cars: vec![2, 4, 6] };
    network.add_mission(mission1);
    network.dispatch_train_across_network(1);
    // network.dispatch_train_across_network(&1);

    if let Some(tidmouth) = network.get_mut_station("Tidmouth"){
        tidmouth.print_status();
    }
    if let Some(brendam_docks) = network.get_mut_station("Brendam Docks"){
        brendam_docks.print_status();
    }

}








