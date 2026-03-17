mod models;

use crate::models::{Cargo, EngineType, TrainError, Engine, TrainCar, Train, Mission, RejectedAsset};

use core::net;
use std::{collections::{HashMap, VecDeque}, u32};

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";

// This program demonstrates the physics of Rust and Networking in the context of a train yard on the Island of Sodor. It models the interactions between trains, engines, cars, cargo, and stations, while also showcasing how ownership and borrowing work in Rust to manage complex state and ensure memory safety. The program also includes a simple representation of a railway network with tracks connecting different stations, allowing for the dispatch and routing of trains across the island. Through this simulation, we can explore how Rust's unique features enable us to build a robust and efficient system that mimics real-world logistics and transportation challenges.


struct Railyard {
    trains: Vec<Train>,
    cars: HashMap<u32, TrainCar>,
    next_train_id: u32,
    purgatory: Vec<RejectedAsset>,
    //cargo: Vec<Cargo>,

}


struct Roundhouse {
    stalls: HashMap<EngineType, VecDeque<Engine>>,
}


struct Station {
    name: String,
    yard: Railyard,
    roundhouse: Roundhouse,
}

struct RailwayNetwork {
    // Maps (Origin, Destination) -> Distance in km
    tracks: HashMap<(String, String), u32>,
    // The network also needs to hold the Stations themselves so it can route trains between them
    stations: HashMap<String, Station>,
    missions: HashMap<u32, Mission>, // <-- The Source of Truth for all missions on the network
}

impl Railyard {
    

    fn new() -> Self {
        Railyard {
            trains: Vec::new(),
            cars: HashMap::new(),
            next_train_id: 1,
            purgatory: Vec::new(),
            //cargo: Vec::new(),
        }
    }

    fn generate_new_id(&mut self) -> u32 {
        let id = self.next_train_id;
        self.next_train_id += 1; // Increment for the next train
        id
    }
    

    pub fn print_report(&self, roundhouse: &Roundhouse) { // <-- Note the new parameter!
        println!("\n{BOLD}{CYAN}┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓{RESET}");
        println!("{BOLD}{CYAN}┃              SODOR RAILWAY: YARD REPORT               ┃{RESET}");
        println!("{BOLD}{CYAN}┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛{RESET}");

        // 1. ACTIVE MISSIONS (Trains)
        println!("  {BOLD}ACTIVE MISSIONS (Assembled Trains){RESET}");
        if self.trains.is_empty() {
            println!("    (No active trains on the line)");
        } else {
            for train in &self.trains {
                let load = train.calculate_cargo_weight();
                println!("    {GREEN}🚂 [TRAIN {:02}]{RESET} | Engine: {:?} (ID: {}) | Cars: {} | Load: {}kg", 
                    train.id, train.engine.engine_type, train.engine.id, train.cars.len(), load);
            }
        }

        // 2. THE MAIN YARD (The Lockers)
        println!("\n  {BOLD}MAIN YARD LOCKERS ({}/100 capacity used){RESET}", self.cars.len()); 
        if self.cars.is_empty() {
            println!("    (No cars currently parked)");
        } else {
            for (id, car) in &self.cars {
                let cargo_desc = match &car.cargo {
                    Some(c) => format!("{} ({}kg)", c.item, c.actual_weight),
                    None => "Empty".to_string(),
                };
                let pax = car.passenger.as_deref().unwrap_or("None");
                println!("    {CYAN}[CAR ID: {:02}]{RESET} | Pax: {:<10} | Cargo: {}", id, pax, cargo_desc);
            }
        }

        // 3. THE PURGATORY (The Stray Track)
        println!("\n  {BOLD}{RED}PURGATORY SIDING (Stray/Invalid Cars){RESET}");
        if self.purgatory.is_empty() {
            println!("    (Clear - All cars accounted for)");
        } else {
            for car in &self.purgatory {
                println!("    {RED}⚠️ [CAR ID: {:02}] | REJECTED | Reason: {:?} | Timestamp: {:?} | Source Mission: {:?}{RESET}", car.car.id, car.issue, car.timestamp, car.source_mission);
            }
        }

        // 4. THE ROUNDHOUSE (Engine Standby)
        println!("\n  {BOLD}ROUNDHOUSE (Engines on Standby){RESET}");
        if roundhouse.stalls.is_empty() {
            println!("    (Roundhouse is empty)");
        } else {
            for (etype, queue) in &roundhouse.stalls {
                if queue.is_empty() { continue; } // Skip empty stalls
                println!("    [{:?}] Stall - {} Engine(s) Waiting:", etype, queue.len());
                for (i, engine) in queue.iter().enumerate() {
                    println!("      {}. Engine {} | Fuel: {:?}", i + 1, engine.id, engine.current_fuel);
                }
            }
        }
        
        println!("{BOLD}{CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{RESET}\n");
    }


    pub fn receive_car(&mut self, mut car: TrainCar) -> Result<(), (TrainCar, TrainError)> {
        // 1. Explicit Check: No duplicate IDs
        if self.cars.contains_key(&car.id) {
            println!("{RED}Railyard Error: Car ID {} is a duplicate!{RESET}", car.id);
            let car_id = car.id;
            return Err((car, TrainError::DuplicateId(car_id)));
        }

        // 2. The Confiscation Check (The Security Gate)
        // We use &mut because we might need to physically alter the cargo
        if let Some(cargo) = &mut car.cargo {
            // We ask the cargo to check itself and confiscate if necessary
            if let Err(e) = cargo.check_and_confiscate() {
                // If the cargo returns a Contraband error, we reject the whole car
                println!("{RED}SECURITY ALERT: Car {} contained illegal goods! Moving to Purgatory.{RESET}", car.id);
                return Err((car, e)); 
            }
        }

        // 3. Success: The state change is clear
        println!("{GREEN}Railyard: Car {} safely docked in locker.{RESET}", car.id);
        self.cars.insert(car.id, car);
        Ok(())
    }


    /// Move a car identified by its `car_id` from the yard into a train.
    ///
    /// Takes ownership of the car by removing it from `self.cars` and pushing it
    /// into `train.cars`.  This avoids double-moving the same `TrainCar` value
    /// (which is what caused the compiler errors you saw earlier).
    
    pub fn couple_by_id(&mut self, train: &mut Train, id: u32) {
        // 1. Look into the 'Locker Room' (HashMap) and try to remove the car
        // 2. We use &id because .remove() only needs to "look" at the key
        if let Some(car) = self.cars.remove(&id) {
            println!("RailYard: Coupling Car {} to Train {}.", id, train.id);
            
            // 3. Physically move that car into the Train's linear track (Vec)
            train.cars.push(car);
        } else {
            println!("RailYard Error: Car {} not found in the yard!", id);
        }
    }

    pub fn decouple_by_id(&mut self, train: &mut Train, id: u32){
        if let Some(pos) = train.cars.iter().position(|c| c.id == id) {
            let car = train.cars.remove(pos);

            if let Err((car, error)) = self.receive_car(car) {
                println!("Failed to return Car {} to the yard: {:?}. Moving to purgatory.", car.id, error);
                let rejected_asset: RejectedAsset = RejectedAsset::new(car, error, 0, train.mission_id); // We can fill in the timestamp and source_mission later when we implement those features.
                self.purgatory.push(rejected_asset);
            }

        } else {
            println!("Car {} is not attached to Train {}.", id, train.id);
        }
    }

    pub fn get_total_cargo_weight(&self, mission: &Mission) -> Result<u32, TrainError> {
        // We extract the data we need from the mission
        let car_ids = &mission.required_cars;
        let mut missing_ids = Vec::new(); // Create a ledger for failures
        let mut total_weight = 0;

        for id in car_ids {
            match self.cars.get(id) {
                Some(car) => total_weight += car.calculate_cargo_weight(),
                None => missing_ids.push(*id), // Log it, but keep checking!
            }
        }

        if !missing_ids.is_empty() {
            return Err(TrainError::AssemblyFailed { 
                missing_car_ids: missing_ids, 
                engine_returned: 0 
            });
        }
        else {
            Ok(total_weight)
        }
    }


    pub fn assemble_cars(&mut self, mission: &Mission /* <--- We take a reference to the work order */) -> Result<Vec<TrainCar>, TrainError> {

        let car_ids = &mission.required_cars;
        
        // 1. Take ownership of the power // Gathering the payload: We have already confirmed that all requested cars exist and that the engine can handle the weight, 

        //MOOWAHAHA! Functional programming style are belong to me! (with Copilot's and Gemini's help of course)
        let attached_cars: Vec<TrainCar> = car_ids.iter()
            // We use .unwrap() fearlessly because the prior loop guarantees existence.
            .map(|id| self.cars.remove(id).unwrap()) 
            .collect();

        Ok(attached_cars)
    }

    // fn disassemble_train(&mut self, train:Train, roundhouse:&mut Roundhouse){
    //     let engine = train.engine;
    //     roundhouse.house(engine);

    //     for car in train.cars {
    //         if let Err((car, error)) = self.receive_car(car) {
    //             println!("Failed to return Car {} to the yard: {:?}. Moving to purgatory.", car.id, error);
    //             self.purgatory.push(car);
    //         }


    //     }
    // }

    pub fn disassemble_train(&mut self, train: Train, roundhouse: &mut Roundhouse) {
        let (engine, cars, _id) = (train.engine, train.cars, train.id); // Destructure the "Gestalt"

        // 1. Return the Power
        roundhouse.house(engine);

        // 2. Process the Cars
        for mut car in cars {
            // Step A: The Security Gate & Intake
            // This handles contraband and duplicate ID checks.
            let car_id_we_just_received = car.id; // Store the ID before we potentially move the car into purgatory
            if let Ok(_) = self.receive_car(car) {
                // Step B: Fulfillment
                // Now that the car is safely in the yard's HashMap, 
                // we can reach in and "deliver" the goods.
                if let Some(mut car_in_yard) = self.cars.get_mut(&car_id_we_just_received) {
                    let payload = car_in_yard.unload_cargo();
                    // Future: Send 'payload' to Warehouse
                }
            } else {
                // receive_car already handles Purgatory internally in your current code.
            }
        }
    }

}   




impl Roundhouse {
    pub fn new() -> Self {
        Roundhouse {
            stalls: HashMap::new(),
        }
    }

    /// Houses an engine in the appropriate stall based on its type.
    pub fn house(&mut self, engine: Engine) {
        self.stalls
            .entry(engine.engine_type) // 1. Check the stall for this EngineType
            .or_insert_with(VecDeque::new)  // 2. If it doesn't exist, build a new track (VecDeque)
            .push_back(engine);             // 3. Park the engine on the track
    }

    pub fn dispatch(&mut self, etype: EngineType) -> Option<Engine> {
        self.stalls
            .get_mut(&etype)? // Find the stall
            .pop_front()      // Take the one that's been waiting longest
    }

    pub fn find_suitable_engine(&mut self, total_weight: u32, distance_km: u32) -> Option<Engine> {
        
        // 1. The Escalation Roster (Weakest to Strongest)
        let roster = [
            EngineType::Percy, 
            EngineType::Thomas, 
            EngineType::Diesel, 
            EngineType::Gordon
        ];

        // 2. Iterate through the roster in order
        for etype in roster {
            // Check if this TYPE is physically strong enough
            if etype.max_capacity() >= total_weight {
                
                // If it is, look inside that specific stall
                if let Some(queue) = self.stalls.get_mut(&etype) {
                    
                    // 1. Find the position of the first capable engine
                    let winner_index = queue.iter().position(|engine| {
                        engine.can_complete_mission(total_weight, distance_km)
                    });

                    // 2. Chain it using the `.and_then()` you love!
                    // If position returned Some(index), and_then passes that index into queue.remove()
                    return winner_index.and_then(|index| queue.remove(index));
                }

            }
        }
        
        // If we loop through the whole roster and find nothing, return None.
        None
    }
}



impl Station {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            yard: Railyard::new(),
            roundhouse: Roundhouse::new(),
        }
    }


    pub fn assemble_and_dispatch(&mut self, mission: &Mission, distance: u32) -> Result<Train, TrainError> {
        println!("{BOLD}{CYAN}[{}] Orchestrating Assembly for Mission {}...{RESET}", self.name, mission.id);

        let required_ids = &mission.required_cars;

        //1/ We can combine the inventory check and weight calculation into a single method in the yard for cleaner code and better separation of concerns.
        let total_weight = self.yard.get_total_cargo_weight(mission)?;


        // 2. Ask the Roundhouse for an Engine (Mutable, as we remove it)
        let engine = self.roundhouse.find_suitable_engine(total_weight, distance)
            .ok_or(TrainError::NoAvailableEngine)?;

        // 3. Command the Yard to extract the cars (Mutable, as we remove them)
        // Since we already proved they exist in Step 1, we can fearlessly unwrap.
        let attached_cars: Vec<TrainCar> = self.yard.assemble_cars(mission)?; // This will also return an error if something goes wrong during the actual assembly process, which is a nice safety net.

        // 4. The Station builds the Gestalt
        let train = Train {
            id: self.yard.generate_new_id(), // Station asks Yard for a tracking ID
            engine,
            cars: attached_cars,
            distance_km: distance,
            mission_id: Some(mission.id),
        };

        Ok(train)
    }

    pub fn receive_car(&mut self, car: TrainCar) {
        
            let car_id = car.id;
        match self.yard.receive_car(car) {
            Ok(_) => println!("Car {} successfully received into the yard.", car_id),
            Err((homeless_car, error)) => {
                println!("Intake failed for Car {}: {:?}. Moving to purgatory.", homeless_car.id, error);
                let rejected_asset = RejectedAsset::new(homeless_car, error, 0, None); // We can fill in the timestamp and source_mission later when we implement those features.
                self.yard.purgatory.push(rejected_asset);
            }
        }
    }

    pub fn house_engine(&mut self, engine: Engine) {
        println!("Roundhouse: Housing Engine {} of type {:?}.", engine.id, engine.engine_type);
        self.roundhouse.house(engine);
    }

    pub fn receive_train(&mut self, train: Train) {
        println!("{BOLD}{GREEN}[{}] Train {} has arrived. Initiating breakdown.{RESET}", self.name, train.id);
        self.yard.disassemble_train(train, &mut self.roundhouse);
    }
    
    // A helper to inspect the local state
    pub fn print_status(&self) {
        println!("\n--- STATION REPORT: {} ---", self.name);
        self.yard.print_report(&self.roundhouse);
    }
}

impl RailwayNetwork {
    pub fn new() -> Self {
        RailwayNetwork {
            tracks: HashMap::new(),
            stations: HashMap::new(),
            missions: HashMap::new(),
        }
    }
    
    pub fn add_station(&mut self, station: Station) {
        // station.name is a String. We can clone it to use as the key, 
        // and move the actual station into the value.
        self.stations.insert(station.name.clone(), station);
    }

    pub fn add_tracks(&mut self, origin: &Station, destination: &Station, distance: u32) {
        // Clone the names to own them inside the tuple key
        let route = (origin.name.clone(), destination.name.clone());
        self.tracks.insert(route, distance);
        
        // Sodor is not a one-way street. You likely want the reverse route too!
        let return_route = (destination.name.clone(), origin.name.clone());
        self.tracks.insert(return_route, distance);
    }

    pub fn add_mission(&mut self, mission: Mission) {
        println!("{YELLOW}Network Ledger: Registered Mission {}.{RESET}", mission.id);
        self.missions.insert(mission.id, mission);
    }

    pub fn get_distance(&self, origin: &Station, destination: &Station) -> Option<u32> {
        self.tracks.get(&(origin.name.clone(), destination.name.clone())).copied()
    }

    /// Allows the global environment (main) to temporarily borrow a station to mutate it.
    pub fn get_mut_station(&mut self, name: &str) -> Option<&mut Station> {
        self.stations.get_mut(name)
    }

    pub fn get_mission(&self, mission_id: &u32) -> Option<&Mission> {
        self.missions.get(mission_id)
    }

    // Note: passing primitive integers by reference (&u32) is unidiomatic. Just pass u32!
    pub fn dispatch_train_across_network(&mut self, mission_id: u32) {
        
        // 1. Direct Field Access. 
        // This immutably locks ONLY self.missions. self.stations is still free!
        let mission = match self.missions.get(&mission_id) {
            Some(m) => m,
            None => {
                println!("{RED}Network Error: Mission {} does not exist in the ledger.{RESET}", mission_id);
                return; 
            }
        };

        // We clone the names here because we will need to re-insert the stations later
        // and we cannot move data out of our borrowed mission reference.
        let origin_name = mission.origin.clone();
        let dest_name = mission.destination.clone();

        // 2. Amputate the Origin (mutably borrowing ONLY self.stations)
        let mut origin = match self.stations.remove(&origin_name) {
            Some(s) => s,
            None => {
                println!("{RED}Network Error: Origin station '{}' not found.{RESET}", origin_name);
                return;
            }
        };

        // 3. Amputate the Destination
        let mut destination = match self.stations.remove(&dest_name) {
            Some(s) => s,
            None => {
                println!("{RED}Network Error: Destination station '{}' not found.{RESET}", dest_name);
                self.stations.insert(origin_name, origin); // Put the origin back before aborting!
                return;
            }
        };

        // 4. Calculate Distance
        let distance = self.get_distance(&origin, &destination).unwrap_or(0);
        println!("Distance from {} to {} is {} km", origin.name, destination.name, distance);

        // 5. The Execution (We pass our single, original `mission` reference!)
        if let Ok(mut train) = origin.assemble_and_dispatch(mission, distance) {
            if let Ok(_) = train.dispatch() {
                println!("{GREEN}Mission {} completed successfully! Train {} has arrived at {}.{RESET}", mission.id, train.id, destination.name);
                destination.receive_train(train);
            } else {
                println!("{RED}TRAIN NOT RECEIVED: Mission {} from {} to {} failed during traversal.{RESET}", mission.id, origin.name, destination.name);
            }
        } else {
            println!("{RED}Mission {} from {} to {} failed to assemble.{RESET}", mission.id, origin.name, destination.name);
        }

        // 6. Return ownership back to the network
        self.stations.insert(origin_name, origin); 
        self.stations.insert(dest_name, destination);
    }

    // pub fn dispatch_train_across_network(&mut self, mission_id: &u32) {

    //     let origin_name: String;
    //     let destination_name: String;
    //     if let Some(m) = self.get_mission(mission_id) {
    //         origin_name = m.origin.clone();
    //         destination_name = m.destination.clone();
    //     } else {
    //         println!("{RED}Network Error: Mission {} does not exist in the ledger.{RESET}", mission_id);
    //         return; // Abort before touching any stations
    //     }
    //     let mut origin = self.stations.remove(&origin_name).expect("Origin station not found");
    //     let mut destination = self.stations.remove(&destination_name).expect("Destination station not found");

    //     // let distance: u32 = self.tracks.get(&(origin.name.clone(), destination.name.clone()))
    //     //     .copied()
    //     //     .unwrap_or(0);

    //     // println!("Dispatching mission {} from {} to {} ({} km)", mission_id, origin.name, destination.name, distance);

    //     let distance = self.get_distance(&origin, &destination).unwrap_or(0);
    //     println!("Distance from {} to {} is {} km", origin.name, destination.name, distance);
    //     if let Some(mission) = self.get_mission(mission_id) {
    //         if let Ok(mut train) = origin.assemble_and_dispatch(mission, distance) {
    //             if let Ok(_) = train.dispatch() {
    //                 println!("{GREEN}Mission {} completed successfully! Train {} has arrived at {}.{RESET}", mission.id, train.id, destination.name);
    //                 destination.receive_train(train); // The train arrives at the destination station, which triggers the disassembly process.
    //             } else {
    //                 println!("{RED}TRAIN NOT RECEIVED:Mission {} from {} to {} failed to dispatch during traversal.{RESET}", mission.id, origin.name, destination.name);
    //             }
    //         } else {
    //             println!("{RED}Mission {} from {} to {} failed to dispatch.{RESET}", mission.id, origin.name, destination.name);
    //         }
    //     }
    //     self.stations.insert(origin.name.clone(), origin); // Return ownership back to the network
    //     self.stations.insert(destination.name.clone(), destination);
    // }

}

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















//TODO::TODO::TODO::TODO::TODO::TODO::TODO::TODO::TODO::TODO::TODO::TODO::TODO::TODO::TODO::TODO::TODO::TODO::TODO::TODO::TODO::TODO::TODO::TODO::
/*


    struct RailwayNetwork {
        tracks: HashMap<(String, String), u32>,
        stations: HashMap<String, Station>,
        mission_ledger: HashMap<u32, Mission>, // <-- The Source of Truth
    }

    impl RailwayNetwork {
        pub fn new() -> Self {
            RailwayNetwork {
                tracks: HashMap::new(),
                stations: HashMap::new(),
                mission_ledger: HashMap::new(),
            }
        }

        pub fn add_mission(&mut self, mission: Mission) {
            println!("{YELLOW}Network Ledger: Registered Mission {}.{RESET}", mission.id);
            self.mission_ledger.insert(mission.id, mission);
        }
        // ... (Keep your add_station, add_tracks, etc.)
    }









    pub fn dispatch_train_across_network(&mut self, origin_name: &str, dest_name: &str, mission_id: u32) {
        1. Consult the Ledger FIRST.
        let mission = match self.mission_ledger.get(&mission_id) {
            Some(m) => m,
            None => {
                println!("{RED}Network Error: Mission {} does not exist in the ledger.{RESET}", mission_id);
                return; // Abort before touching any stations
            }
        };

        2. Safely amputate the Origin.
        let mut origin = match self.stations.remove(origin_name) {
            Some(s) => s,
            None => {
                println!("{RED}Network Error: Origin station '{}' not found.{RESET}", origin_name);
                return;
            }
        };

        3. Safely amputate the Destination.
        let mut destination = match self.stations.remove(dest_name) {
            Some(s) => s,
            None => {
                println!("{RED}Network Error: Destination station '{}' not found.{RESET}", dest_name);
                self.stations.insert(origin.name.clone(), origin); // Put the origin back! 
                return;
            }
        };

        4. Calculate Distance
        let distance = self.get_distance(&origin, &destination).unwrap_or(0);
        println!("{BOLD}{CYAN}--- Dispatching Mission {} from {} to {} ({} km) ---{RESET}", mission_id, origin_name, dest_name, distance);

        5. The Physics of the Traversal
        if let Ok(mut train) = origin.dispatch_train(mission, distance) {
            if let Ok(_) = train.dispatch() {
                println!("{GREEN}Mission {} completed successfully! Train {} has arrived at {}.{RESET}", mission.id, train.id, dest_name);
                destination.receive_train(train);
            }
        } else {
            println!("{RED}Mission {} from {} to {} failed to assemble/dispatch.{RESET}", mission.id, origin_name, dest_name);
        }

        6. Re-attach the Gestalt
        self.stations.insert(origin.name.clone(), origin);
        self.stations.insert(destination.name.clone(), destination);
    }










    let mission1 = Mission { id: 1, destination: String::from("Brendam Docks"), required_cars: vec![2, 4, 6] };
    
    network.add_mission(mission1); // Hand the mission to the Network

    // You now only pass the primitive u32 ID!
    network.dispatch_train_across_network("Tidmouth", "Brendam Docks", 1);

*/