use crate::models::{Train, TrainCar, Engine, Mission, TrainError, RejectedAsset, EngineType, Cargo, Location, MissionReport};
use crate::network::RailwayNetwork;
use std::collections::{HashMap, VecDeque};
use rand::Rng;

use std::sync::Arc;
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;
use crate::models::StationCommand;

// (Don't forget to paste your color constants here too, or put them in a shared module later)
const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";


















pub struct Railyard {
    pub trains: Vec<Train>,
    pub cars: HashMap<u32, TrainCar>,
    pub next_train_id: u32,
    pub purgatory: Vec<RejectedAsset>,
}


impl Railyard {
    

    fn new() -> Self {
        Railyard {
            trains: Vec::new(),
            cars: HashMap::new(),
            next_train_id: 1,
            purgatory: Vec::new(),
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


    pub fn receive_car(&mut self, mut car: TrainCar) -> Result<Option<Cargo>, (TrainCar, Vec<TrainError>)> {

        let mut issues = Vec::<TrainError>::new();

        // 1. Explicit Check: No duplicate IDs
        if self.cars.contains_key(&car.id) || self.purgatory.iter().any(|asset| asset.car.id == car.id) { // tell me about the .any operator please, Copilot. .any() is a method that checks if any element in the iterator satisfies a given condition. In this case, we're using it to check if any car in purgatory has the same ID as the incoming car. If it finds a match, it returns true, which means we have a duplicate ID situation. This is important because we want to prevent two different cars from having the same ID in our system, which could cause confusion and errors down the line.
            println!("{RED}Railyard Error: Car ID {} is a duplicate!{RESET}", car.id);
            let car_id = car.id;
            issues.push(TrainError::DuplicateId(car_id));
        }

        // 2. The Confiscation Check (The Security Gate)
        // We use &mut because we might need to physically alter the cargo
        if let Some(cargo) = &mut car.cargo {
            // We ask the cargo to check itself and confiscate if necessary
            if let Err(e) = cargo.check_and_confiscate() {
                // If the cargo returns a Contraband error, we reject the whole car
                println!("{RED}SECURITY ALERT: Car {} contained illegal goods! Moving to Purgatory.{RESET}", car.id);
                issues.push(e);
            }
        }

        if issues.is_empty() {
            // 3. Success: The state change is clear
            println!("{GREEN}Railyard: Car {} safely docked in locker.{RESET}", car.id);
            let cargo = car.cargo.take(); // We want to pass the cargo up to the warehouse, but we also want to keep the car in the yard's inventory. By using .take(), we move the cargo out of the car and replace it with None, which allows us to return the cargo to the caller while still keeping the car in our HashMap for future reference.
            self.cars.insert(car.id, car);
            Ok(cargo)
        }
        else {
            // 4. Failure: We return the car and the ledger of issues for transparency and debugging.
            Err((car, issues))
        }
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

            if let Err((car, issues)) = self.receive_car(car) {
                println!("Failed to return Car {} to the yard: {:?}. Moving to purgatory.", car.id, issues);
                let rejected_asset: RejectedAsset = RejectedAsset::new(car, issues, train.mission_id); // We can fill in the timestamp and source_mission later when we implement those features.
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
            println!("{RED}Yard: Total cargo weight for Mission {} is {}kg.{RESET}", mission.id, total_weight);
            return Err(TrainError::AssemblyFailed { 
                missing_car_ids: missing_ids, 
                engine_returned: 0 
            });
        }
        else {
            Ok(total_weight)
        }
    }


    pub fn assemble_cars(&mut self, mission: &Mission) -> Result<Vec<TrainCar>, TrainError> {
        let car_ids = &mission.required_cars;
        
        // We use .unwrap() fearlessly because the Station already guaranteed existence with get_total_cargo_weight()! If we made it here, we know all the cars exist, so any failure at this point would be a critical error that should panic the system, because it means our internal state is inconsistent. By using .unwrap(), we allow such a critical error to surface immediately during development/testing, rather than silently failing or returning an error that we didn't expect to have to handle.
        // This is much faster and doesn't require complex rollback logic.
        let attached_cars: Vec<TrainCar> = car_ids.iter()
            .map(|id| self.cars.remove(id).expect("Inventory validation failed prior to assembly")) 
            .collect();

        Ok(attached_cars)
    }

    pub fn disassemble_train(&mut self, train: Train, roundhouse: &mut Roundhouse) -> Result<Vec<Cargo>, TrainError> {
        let (engine, cars, id, mission_id) = (train.engine,train.cars, train.id, train.mission_id); // Destructure the "Gestalt"

        // 1. Return the Power
        roundhouse.house(engine);

        // 2. Process the Cars
        let mut returned_cargo = Vec::new();
        for car in cars {
            // Step A: The Security Gate & Intake
            // This handles contraband and duplicate ID checks.
            let car_id_we_just_received = car.id; // Store the ID before we potentially move the car into purgatory
            match self.receive_car(car) {
                Ok(_) => {
                    // Step B: Fulfillment
                    // Now that the car is safely in the yard's HashMap, 
                    // we can reach in and "deliver" the goods.
                    if let Some(car_in_yard) = self.cars.get_mut(&car_id_we_just_received) {
                        let payload = car_in_yard.unload_cargo();
                        // Future: Send 'payload' to Warehouse
                        if let Some(cargo) = payload {
                            println!("{GREEN}Train {}: Successfully delivered cargo '{}' from Car {} to the yard.{RESET}", id, cargo.item, car_id_we_just_received);
                            returned_cargo.push(cargo);
                        } else {
                            println!("{YELLOW}Train {}: Car {} had no cargo to unload.{RESET}", id, car_id_we_just_received);
                        }
                    }   
                } 
                Err((homeless_car, e)) => {
                    println!("{RED}Train {}: Failed to process Car {} during disassembly: {:?}. Moving to purgatory.{RESET}", id, car_id_we_just_received, e);
                    // Note: If train.mission_id is already an Option, you might just be able to pass it directly 
                    // without wrapping it in Some() and unwrap_or(0), depending on your RejectedAsset signature!
                    let rejected_asset = RejectedAsset::new(homeless_car, e, mission_id); //we do not need to unwrap_or(0) here because mission_id is already an Option in the Train struct, so we can pass it directly
                    self.purgatory.push(rejected_asset);
                }
            }
        }

        Ok(returned_cargo)
    }   
}


pub struct Roundhouse {
    pub stalls: HashMap<EngineType, VecDeque<Engine>>,
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

    pub fn find_suitable_engine(&mut self, total_weight: u32, distance_km: f64) -> Result<Engine, TrainError> {
        
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
                println!("{YELLOW}Roundhouse: Checking for available {:?} engines...{RESET}", etype);
                
                // If it is, look inside that specific stall
                if let Some(queue) = self.stalls.get_mut(&etype) {
                    
                    // 1. Find the position of the first capable engine
                    let winner_index = queue.iter().position(|engine| {
                        engine.can_complete_mission(total_weight, distance_km)
                    });

                    // 2. Chain it using the `.and_then()` you love!
                    // If position returned Some(index), and_then passes that index into queue.remove()
                    if let Some(engine) = winner_index.and_then(|index| queue.remove(index)) {
                        println!("{GREEN}Roundhouse: Dispatching Engine {} of type {:?} for mission ({}kg over {}km).{RESET}", engine.id, engine.engine_type, total_weight, distance_km);
                        return Ok(engine);
                    }
                }

            }
        }
        
        // If we loop through the whole roster and find nothing, return an error.
        println!("{RED}Roundhouse: No suitable engines available for mission ({}kg over {}km).{RESET}", total_weight, distance_km);
        Err(TrainError::MissionImpossible { reason: "NO ENGINES CAN COMPLETE MISSION!".to_string() })
    }
}




pub struct Warehouse {
    pub inventory: Vec<Cargo>,
}

impl Warehouse {
    pub fn new() -> Self {
        Warehouse {
            inventory: Vec::new(),
        }
    }

    pub fn store(&mut self, cargo: Cargo) {
        println!("{BOLD}{YELLOW}Warehouse: Received {} ({}kg) for processing/holding.{RESET}", cargo.item, cargo.actual_weight);
        self.inventory.push(cargo);
    }

    pub fn process_outbound(&mut self) {
        // This represents fulfillment to the "outside world"
        let fulfilled = self.inventory.len();
        self.inventory.clear();
        if fulfilled > 0 {
            println!("{BOLD}{GREEN}Warehouse: Successfully processed and delivered {} cargo shipments to the outside world.{RESET}", fulfilled);
        }
    }
}
















































pub struct StationState {
    pub name: String,
    pub yard: Railyard,
    pub roundhouse: Roundhouse,
    pub warehouse: Warehouse,
    pub map: Arc<RailwayNetwork>,
    pub tx: Sender<StationCommand>, // The Boomerang
}


impl StationState {
    pub fn new(name: String, map: Arc<RailwayNetwork>, tx: Sender<StationCommand>) -> Self {
        StationState {
            name,
            yard: Railyard::new(),
            roundhouse: Roundhouse::new(),
            warehouse: Warehouse::new(),
            map,
            tx,
        }
    }





















    // The VIP Pass is `&mut self`. This allows the method to open its own briefcase!
    pub fn handle_assemble_mission(
        &mut self, 
        mission: Mission, 
        //distance: f64, 
        //route: Vec<String>, 
        //destination: String, 
        reply_to: Sender<Result<Train, TrainError>>
    ) {
        println!("{BOLD}{CYAN}[{}] Received command to assemble mission {}.{RESET}", self.name, mission.id);
        
        // 1. Paste your assemble_train logic here!
        // First, we need to use our map! Choo choo!



        let (distance, route) = match self.map.find_shortest_path(&self.name, &mission.destination) {
            Some((d, r)) => {
                println!(
                    "{YELLOW}Network: Shortest path for Mission {} is {} km via {:?}.{RESET}",
                    mission.id, d, r
                );
                (d, r)
            },
            None => {
                println!("{RED}Network Error: No track laid between {} and {}.{RESET}", self.name, mission.destination);
                let error = TrainError::MissionImpossible { reason: "Destination unreachable".to_string() };
                if reply_to.send(Err(error)).is_err() {
                    println!("{RED}[{}] DEAD-LETTER: Failed to send assembly failure for mission {} due to unreachable destination.{RESET}", self.name, mission.id);
                }
                return;
            }
        };

        // At this point, we have the distance and route calculated, so we can proceed with the assembly logic using these values.

        // Now for the fun part: we're going to completely rewrite assemble_train as part of the Station's responsibilities, because the Station is now the mastermind behind the whole operation, and it needs to have access to its internal state (the yard and roundhouse) to pull this off. The network is just a map and dispatcher, so it makes more sense for the Station to handle the assembly logic directly.
        
        let total_weight = match self.yard.get_total_cargo_weight(&mission) {
            Ok(weight) => weight,
            Err(e) => {
                println!("{RED}Yard Error: Failed to calculate total cargo weight for Mission {}: {:?}.{RESET}", mission.id, e);
                // if reply_to.send(Err(e)).is_err() {
                //     println!("{RED}[{}] DEAD-LETTER: Failed to send assembly failure for mission {} due to cargo weight calculation error.{RESET}", self.name, mission.id);
                // }
                self.send_mission_failure(mission.id, e, reply_to);
                return;
            }
        };


        let engine = match self.roundhouse.find_suitable_engine(total_weight, distance) {
            Ok(engine) => engine,
            Err(e) => {
                println!("{RED}Roundhouse Error: Failed to find suitable engine for Mission {}: {:?}.{RESET}", mission.id, e);
                // if reply_to.send(Err(e)).is_err() {
                //     println!("{RED}[{}] DEAD-LETTER: Failed to send assembly failure for mission {} due to engine availability error.{RESET}", self.name, mission.id);
                // }
                self.send_mission_failure(mission.id, e, reply_to);
                return;
            }
        };


        let attached_cars = match self.yard.assemble_cars(&mission) {
            Ok(cars) => cars,
            Err(e) => {
                println!("{RED}Yard Error: Failed to assemble cars for Mission {}: {:?}.{RESET}", mission.id, e);
                // Since we already took the engine out of the roundhouse, we need to return it back to avoid losing it due to a failed assembly!
                self.roundhouse.house(engine);
                // if reply_to.send(Err(e)).is_err() {
                //     println!("{RED}[{}] DEAD-LETTER: Failed to send assembly failure for mission {} due to car assembly error.{RESET}", self.name, mission.id);
                // }
                self.send_mission_failure(mission.id, e, reply_to);
                return;
            }
        };



        let mut train = Train {
            id: self.yard.generate_new_id(),
            engine,
            cars: attached_cars,
            mission_id: Some(mission.id),
            destination: mission.destination.clone(),
            report_to: mission.reply_channel.clone(),
        };

        
        // 2. Paste your Dijkstra routing and thread::spawn transit logic here! NO! Okay, maybe the thread::spawn part can stay

        let next_stop = route.get(1).cloned().unwrap_or_else(|| train.destination.clone()); // The next stop is the second element in the route (index 1), or the destination if the route is just one stop
        let next_stop_handle = self.map.get_station_handle(&next_stop).expect("Next stop must exist in the network").clone();
        let distance_to_next_stop = self.map.get_distance(&self.name, &next_stop).expect("Distance to next stop must be calculable");
        let (transit_tx, transit_rx) = mpsc::channel();

        let train_id = train.id; // Store the train ID for logging inside the thread
        let station_name_clone = self.name.clone(); // Clone the station name for use in this thread
        let my_station_handle = self.tx.clone(); // Clone the station's own Sender for use in the thread, so we can send SOS if needed

        // Remember to use `self.map` to find the shortest path, and `self.tx.clone()` for the SOS radio

        thread::spawn(move || {
            let time = train.dispatch(distance_to_next_stop).expect("Failed to dispatch");
            println!("{BOLD}{YELLOW}[{}] Train {} is en route to next stop {}. Estimated time: {:.2} seconds.{RESET}", station_name_clone, train_id, next_stop, time);
            thread::sleep(std::time::Duration::from_secs_f64(time)); // Simulate travel time to the next station. In a real implementation, this would be based on distance and train speed.

            // Using rand, simulate the train crashing with a 10% chance during transit. If it crashes, we issue a Derailment report back to transit_rx and skip the rest of the transit logic. The train is lost, so we don't send it to the next station. However, we return the salvaged TrainCars back to the yard for processing, and we send a MissionReport::Failure back to the mission's reply channel with details of the crash.
            let tree_falls = rand::thread_rng().gen_bool(0.1);
            if tree_falls {
                println!("{RED}🚨 DERAILMENT!{RESET}");

                // We send an SOS command BACK to the Station's main mailbox!
                // (You will need to pass a clone of the Station's own Sender into the thread)
                my_station_handle.send(StationCommand::HandleEmergencySOS {
                    mission_id: train.mission_id.unwrap_or(0),
                    surviving_cars: train.cars, // The train dies, but the cars live!
                    report_to: train.report_to,
                }).expect("SOS failed");

                return; // Thread ends. Engine drops. The cars are now in limbo until the station processes the SOS and returns them to the yard or purgatory.
            } else {
                println!("{GREEN}{BOLD}[{}] Train {} has successfully arrived at next stop {}. Sending receive command...{RESET}", station_name_clone, train_id, next_stop);
                next_stop_handle.send(StationCommand::ReceiveTrain { train, reply_to: transit_tx }).expect("Failed to forward train to next station");
            }

            match transit_rx.recv() {
                Ok(_) => {
                    println!("{BOLD}{CYAN}[{}] CHOO CHOO! Train {} has been received at {}. Finalizing transit...{RESET}", station_name_clone, train_id, next_stop);
                    // Here we would handle the result of the transit, such as sending a MissionReport back to the mission's reply channel based on success or failure at the next station.
                },
                Err(e) => {
                    println!("{RED}[{}] ERROR receiving transit confirmation for Train {}: {:?}{RESET}", station_name_clone, train_id, e);
                }
            }
        });
    }



















    pub fn handle_receive_train(&mut self, mut train: Train, reply_to: Sender<Result<(), TrainError>>) {
        println!("{GREEN}[{}] Processing arrival of Train {}.{RESET}", self.name, train.id);
        let final_destination = &train.destination;
        let current_location = &self.name;
        let station_tx_clone = self.tx.clone(); // Clone the station's own Sender for use in this method, so we can send SOS if needed

        if current_location == final_destination {
            println!("{GREEN}[{}] Train {} has reached its final destination! Unloading...{RESET}", self.name, train.id);
            //crack the egg
            let (engine, cars, id, mission_id, report_to) = (train.engine,train.cars, train.id, train.mission_id, train.report_to); // Destructure the "Gestalt"
            let num_cars = cars.len();
            let mut issues_clone = Vec::new(); // We can fill this with any issues that arise during disassembly, and then include it in the MissionReport for transparency and debugging. For now, we'll just keep it empty to represent a perfect disassembly.
            // 1. Return the Power
            self.roundhouse.house(engine);
            // 2. Return the Cars
            for car in cars {
                // Step A: The Security Gate & Intake
                // This handles contraband and duplicate ID checks.
                let car_id_we_just_received = car.id; // Store the ID before we potentially move the car into purgatory
                match self.yard.receive_car(car) {
                    Ok(Some(cargo)) => {
                        // Step B: Fulfillment
                        // Now that the car is safely in the yard's HashMap, 
                        // we can reach in and "deliver" the goods to the warehouse.
                        self.warehouse.store(cargo);
                    }
                    Ok(None) => {
                        println!("{YELLOW}Train {}: Car {} had no cargo to unload.{RESET}", id, car_id_we_just_received);
                    }
                    Err((homeless_car, e)) => {
                        println!("{RED}Train {}: Failed to process Car {} during disassembly: {:?}. Moving to purgatory.{RESET}", id, car_id_we_just_received, e);
                        // Note: If train.mission_id is already an Option, you might just be able to pass it directly 
                        // without wrapping it in Some() and unwrap_or(0), depending on your RejectedAsset signature!
                        issues_clone.push(homeless_car.id); // Log the ID of the car that caused issues for transparency
                        let rejected_asset = RejectedAsset::new(homeless_car, e, mission_id); //we do not need to unwrap_or(0) here because mission_id is already an Option in the Train struct, so we can pass it directly
                        self.yard.purgatory.push(rejected_asset);
                    }
                }
            }
            let _ = reply_to.send(Ok(())); // Send success back to transit thread so it can peacefully die.
            if issues_clone.is_empty() {
                if let Some(sender) = report_to {
                    let report = MissionReport::Success(format!(
                        "Train {} successfully completed Mission {} by delivering {} cars to {}.",
                        id, mission_id.unwrap_or(0), num_cars, self.name
                    ));
                    let _ = sender.send(report);
                }
            } else {
                if let Some(sender) = report_to {
                    let report = MissionReport::PartialSuccess(format!(
                        "Train {} completed Mission {} by delivering {} cars to {}, but {} cars had issues during disassembly.",
                        id, mission_id.unwrap_or(0), num_cars - issues_clone.len(), self.name, issues_clone.len()
                    ));
                    let _ = sender.send(report);
                }
            }
            // if let Some(sender) = report_to {
            //     let report = MissionReport::Success(format!(
            //         "Train {} successfully completed Mission {} by delivering {} cars to {}.",
            //         id, mission_id, cars.len(), self.name
            //     ));
            //     let _ = sender.send(report);
            // }
        } else {
            let final_destination = &train.destination;
            let current_location = &self.name;
            let (distance, route) = match self.map.find_shortest_path(current_location, final_destination) {
                Some((d, r)) => {
                    println!(
                        "{YELLOW}Network: Shortest path for Train {} to final destination {} is {} km via {:?}.{RESET}",
                        train.id, final_destination, d, r
                    );
                    (d, r)
                },
                None => {
                    println!("{RED}Network Error: No track laid between {} and {}. Cannot forward train.{RESET}", self.name, final_destination);
                    
                    let error = TrainError::MissionImpossible { reason: "Destination unreachable".to_string() };
                    if reply_to.send(Err(error)).is_err() {
                        println!("{RED}[{}] DEAD-LETTER: Failed to send transit failure for Train {} due to unreachable destination.{RESET}", self.name, train.id);
                    }
                    if let Some(sender) = train.report_to {
                        let report = MissionReport::Failure(format!(
                            "Train {} failed to reach final destination {} because it is unreachable from {}.",
                            train.id, final_destination, self.name
                        ));
                        let _ = sender.send(report);
                    }
                    return;
                }
            };

            let next_stop = route.get(1).cloned().unwrap_or_else(|| final_destination.clone()); // The next stop is the second element in the route (index 1), or the final destination if the route is just one stop
            let next_stop_handle = self.map.get_station_handle(&next_stop).expect("Next stop must exist in the network").clone();
            let distance_to_next_stop = self.map.get_distance(&self.name, &next_stop).expect("Distance to next stop must be calculable");

            let train_id = train.id; // Store the train ID for logging inside the thread
            let station_name_clone = self.name.clone(); // Clone the station name for use in this thread
            let (transit_tx, transit_rx) = mpsc::channel();

            thread::spawn(move || {
                let time = train.dispatch(distance_to_next_stop).expect("Failed to dispatch");
                println!("{BOLD}{YELLOW}[{}] Train {} is en route to next stop {}. Estimated time: {:.2} seconds.{RESET}", station_name_clone, train_id, next_stop, time);
                thread::sleep(std::time::Duration::from_secs_f64(time)); // Simulate travel time to the next station. In a real implementation, this would be based on distance and train speed.

                // Using rand, simulate the train crashing with a 10% chance during transit. If it crashes, we issue a Derailment report back to transit_rx and skip the rest of the transit logic. The train is lost, so we don't send it to the next station. However, we return the salvaged TrainCars back to the yard for processing, and we send a MissionReport::Failure back to the mission's reply channel with details of the crash.
                let tree_falls = rand::thread_rng().gen_bool(0.1);
                if tree_falls {
                    println!("{RED}🚨 DERAILMENT!{RESET}");

                    // We send an SOS command BACK to the Station's main mailbox!
                    // (You will need to pass a clone of the Station's own Sender into the thread)
                    station_tx_clone.send(StationCommand::HandleEmergencySOS {
                        mission_id: train.mission_id.unwrap_or(0),
                        surviving_cars: train.cars, // The train dies, but the cars live!
                        report_to: train.report_to,
                    }).expect("SOS failed");

                    return; // Thread ends. Engine drops. The cars are now in limbo until the station processes the SOS and returns them to the yard or purgatory.
                } else {
                    println!("{GREEN}{BOLD}[{}] Train {} has successfully arrived at next stop {}. Sending receive command...{RESET}", station_name_clone, train_id, next_stop);
                    next_stop_handle.send(StationCommand::ReceiveTrain { train, reply_to: transit_tx }).expect("Failed to forward train to next station");
                }

                match transit_rx.recv() {
                    Ok(_) => {
                        println!("{BOLD}{CYAN}[{}] CHOO CHOO! Train {} has been received at {}. Finalizing transit...{RESET}", station_name_clone, train_id, next_stop);
                        // Here we would handle the result of the transit, such as sending a MissionReport back to the mission's reply channel based on success or failure at the next station.
                    },
                    Err(e) => {
                        println!("{RED}[{}] ERROR receiving transit confirmation for Train {}: {:?}{RESET}", station_name_clone, train_id, e);
                    }

                }
            });                      
        }




        // 1. Put the engine back in the roundhouse.
        // 2. Put the cars back in the yard.
        // 3. (Optional) Check if the train arrived at its final destination. If so, mail a success report!
        // 4. Send an Ok(()) back to the transit thread so it can peacefully die.
    }


















    fn send_mission_failure(&self, mission_id: u32, error: TrainError, reply_to: Sender<Result<Train, TrainError>>) {
        if reply_to.send(Err(error)).is_err() {
            println!("{RED}Network Error: Failed to send mission failure report for Mission {}.{RESET}", mission_id);
        }
    }
}



































// This is JUST data. No threads, no channels, no logic.
    pub struct StationMetadata {
        pub name: String,
        pub location: Location,

    }

pub struct Station {
    pub name: String,
    pub tx: Sender<StationCommand>, // The station's command channel for receiving instructions
    pub map: Arc<RailwayNetwork>, // The shared network map for the station to access
    pub location: Location, // The station's location on the network (for distance calculations)
    // We no longer hold the yard, warehouse, and roundhouse directly in the Station struct
}

fn assemble_train(
    station_name: &str,
    yard: &mut Railyard,
    roundhouse: &mut Roundhouse,
    mission: &Mission,
    distance: f64,
    route: Vec<String>,
    destination: String,
) -> Result<Train, TrainError> {
    println!("{BOLD}{CYAN}[{}] Orchestrating Assembly for Mission {}...{RESET}", station_name, mission.id);

    let total_weight = yard.get_total_cargo_weight(mission)?;
    let engine = match roundhouse.find_suitable_engine(total_weight, distance) {
        Ok(engine) => engine,
        Err(e) => {
            println!("{RED}[{}] Assembly failed for Mission {}: {:?}{RESET}", station_name, mission.id, e);
            return Err(TrainError::AssemblyFailed { 
                missing_car_ids: Vec::new(), // We know the cars are there, so this is not a missing car issue
                engine_returned: 0 // No engine was dispatched, so nothing to return
            });
        }
    };
    let attached_cars: Vec<TrainCar> = yard.assemble_cars(mission)?;

    Ok(Train {
        id: yard.generate_new_id(),
        engine,
        cars: attached_cars,
        //distance_km: distance,
        mission_id: Some(mission.id),
        //route_to_destination: route, // This will be filled in by the network when the train is dispatched
        destination: destination, // This will also be filled in by the network
        report_to: mission.reply_channel.clone(), // The train can use this channel to send updates back to the mission
    })
}

fn receive_train_internal(
    station_name: &str,
    yard: &mut Railyard,
    roundhouse: &mut Roundhouse,
    warehouse: &mut Warehouse,
    train: Train,
) -> Result<(), TrainError> {
    println!(
        "{BOLD}{CYAN}[{}] Received an incoming train {}. Initiating breakdown...{RESET}",
        station_name, train.id
    );


    let payloads = yard.disassemble_train(train, roundhouse)?;
    for cargo in payloads {
        warehouse.store(cargo);
    }

    Ok(())
}


impl Station {
    pub fn new(name: &str, map: Arc<RailwayNetwork>, rx: Receiver<StationCommand>) {
        // Create a channel for this station
        // instantiate roundhouse, yard, and warehouse, and copy station name, before moving them into the thread
        let mut roundhouse = Roundhouse::new();
        let mut yard = Railyard::new();
        let mut warehouse = Warehouse::new();
        let station_name = String::from(name);
        let tx = map.get_station_handle(&station_name).expect("Station handle must exist").clone();

        // Spawn a thread to run the station's internal loop
        thread::spawn(move || {
            // The station's internal state
            println!("{BOLD}{CYAN}[{}] Station is now operational and awaiting commands...{RESET}", station_name);

            // The station's main loop
            for command in rx {
                match command {
                    StationCommand::AssembleMission { mission, distance, route, destination, reply_to } => {
                        println!("{BOLD}{CYAN}[{}] Received command to assemble mission {}.{RESET}", station_name, mission.id);
                        match assemble_train(&station_name, &mut yard, &mut roundhouse, &mission, distance, route, destination) {
                            Ok(mut train) => {
                                if let Some((distance, route)) = map.find_shortest_path(&station_name, &train.destination) {
                                    let next_stop = route.get(1).cloned().unwrap_or_else(|| train.destination.clone()); // The next stop is the second element in the route (index 1), or the destination if the route is just one stop
                                    let next_stop_handle = map.get_station_handle(&next_stop).expect("Next stop must exist in the network").clone();
                                    let distance_to_next_stop = map.get_distance(&station_name, &next_stop).expect("Distance to next stop must be calculable");
                                    let (transit_tx, transit_rx) = mpsc::channel();
                                    println!("{BOLD}{GREEN}[{}] Route calculated for Train {}: {}{RESET}", station_name, train.id, route.join(" -> "));

                                    
                                    let train_id = train.id; // Store the train ID for logging inside the thread
                                    


                                    let station_name_clone = station_name.clone(); // Clone the station name for use in this thread
                                    let my_station_handle = tx.clone(); // Clone the station's own Sender for use in the thread, so we can send SOS if needed
                                    // This thread will wait for the train to arrive at the next station and then send a command to that station to receive the train
                                    thread::spawn(move || {
                                        let time = train.dispatch(distance_to_next_stop).expect("Failed to dispatch");
                                        println!("{BOLD}{YELLOW}[{}] Train {} is en route to next stop {}. Estimated time: {:.2} seconds.{RESET}", station_name_clone, train_id, next_stop, time);
                                        thread::sleep(std::time::Duration::from_secs_f64(time)); // Simulate travel time to the next station. In a real implementation, this would be based on distance and train speed.




                                        // Using rand, simulate the train crashing with a 10% chance during transit. If it crashes, we issue a Derailment report back to transit_rx and skip the rest of the transit logic. The train is lost, so we don't send it to the next station. However, we return the salvaged TrainCars back to the yard for processing, and we send a MissionReport::Failure back to the mission's reply channel with details of the crash.
                                        let tree_falls = rand::thread_rng().gen_bool(0.1);
                                        if tree_falls {
                                            println!("{RED}🚨 DERAILMENT!{RESET}");
    
                                            // We send an SOS command BACK to the Station's main mailbox!
                                            // (You will need to pass a clone of the Station's own Sender into the thread)
                                            my_station_handle.send(StationCommand::HandleEmergencySOS {
                                                mission_id: train.mission_id.unwrap_or(0),
                                                surviving_cars: train.cars, // The train dies, but the cars live!
                                                report_to: train.report_to,
                                            }).expect("SOS failed");

                                            return; // Thread ends. Engine drops. The cars are now in limbo until the station processes the SOS and returns them to the yard or purgatory.
                                        } else {
                                            println!("{GREEN}{BOLD}[{}] Train {} has successfully arrived at next stop {}. Sending receive command...{RESET}", station_name_clone, train_id, next_stop);
                                            
                                        next_stop_handle.send(StationCommand::ReceiveTrain { train, reply_to: transit_tx }).expect("Failed to forward train to next station");
                                        }






                                        match transit_rx.recv() {
                                            Ok(result) => {
                                                println!("{BOLD}{CYAN}[{}] CHOO CHOO! Train {} has been received at {}. Finalizing transit...{RESET}", station_name_clone, train_id, next_stop);
                                                // match result {
                                                //     Ok(_) => {
                                                //         println!("{GREEN}[{}] Train {} successfully processed at {}. Transit complete!{RESET}", station_name_clone, train_id, next_stop);
                                                //         let mission_report = MissionReport::Success(format!("Train {} successfully arrived at {}.", train_id, next_stop));
                                                //         if let Some(sender) = train.report_to {
                                                //             sender.send(mission_report).unwrap_or_else(|e| println!("{RED}[{}] Failed to send success report for Train {} after transit: {:?}{RESET}", station_name_clone, train_id, e));
                                                //         }
                                                //     },
                                                //     Err(e) => {
                                                //         println!("{RED}[{}] ERROR processing Train {} at {}: {:?}{RESET}", station_name_clone, train_id, next_stop, e);
                                                //         let mission_report = MissionReport::Failure(format!("Train {} failed to arrive at {}: {:?}", train_id, next_stop, e));
                                                //         if let Some(sender) = train.report_to {
                                                //             sender.send(mission_report).unwrap_or_else(|e| println!("{RED}[{}] Failed to send failure report for Train {} after transit: {:?}{RESET}", station_name_clone, train_id, e));
                                                //         }
                                                //     }
                                                // }

                                            },
                                            Err(e) => {
                                                println!("{RED}[{}] ERROR receiving transit confirmation for Train {}: {:?}{RESET}", station_name_clone, train_id, e);
                                            }
                                        }
                                    });
                                    
                                }

                            },
                            Err(e) => {
                                println!("{RED}[{}] Assembly failed for mission {}: {:?}{RESET}", station_name, mission.id, e);
                                // We attempt to send the failure back to the network so it can handle it (like returning the engine if it was dispatched)
                                if let Err(send_error) = reply_to.send(Err(e)) {
                                    println!(
                                        "{RED}[{}] DEAD-LETTER: Failed to send assembly failure for mission {}. Error: {:?}{RESET}",
                                        station_name, mission.id, send_error
                                    );
                                } else {
                                    println!("{YELLOW}[{}] Successfully sent assembly failure for mission {} back to network.{RESET}", station_name, mission.id);
                                }
                            }
                        }
                    },
                    StationCommand::ReceiveTrain {mut train, reply_to } => {


                        if station_name == train.destination {
                            println!("{BOLD}{GREEN}[{}] Train {} has arrived at its destination! No route needed.{RESET}", station_name, train.id);
                            
                            // let result = receive_train_internal(
                            //     &station_name,
                            //     &mut yard,
                            //     &mut roundhouse,
                            //     &mut warehouse,
                            //     train,
                            // );
                            // if reply_to.send(result).is_err() {
                            //     println!("{RED}[{}] DEAD-LETTER: receive-train reply dropped.{RESET}", station_name);
                            // }
                            let train_id = train.id; // Store the train ID for logging
                            let report_to = train.report_to.clone(); // Clone the report_to sender for use in this thread, so we can send a MissionReport if needed
                            match receive_train_internal(
                                &station_name,
                                &mut yard,
                                &mut roundhouse,
                                &mut warehouse,
                                train,
                            ) {
                                Ok(_) => {
                                    let mission_report = MissionReport::Success(format!("Train {} successfully processed at destination {}.", train_id, station_name));
                                 //   if reply_to.send(Ok(mission_report)).is_err() {
                                    match reply_to.send(Ok(())) {
                                        Ok(_) => println!("{YELLOW}[{}] Successfully sent arrival confirmation for Train {} back to network.{RESET}", station_name, train_id),
                                        Err(e) => println!("{RED}[{}] DEAD-LETTER: Failed to send success confirmation for Train {} at destination: {:?}{RESET}", station_name, train_id, e),
                                    }
                                    // if reply_to.send(Ok(())) .is_err() {
                                    //     println!("{RED}[{}] DEAD-LETTER: Failed to send success confirmation for Train {} at destination.{RESET}", station_name, train_id);
                                    // } else {
                                    //     println!("{YELLOW}[{}] Successfully sent arrival confirmation for Train {} back to network.{RESET}", station_name, train_id);
                                    // }
                                    println!("{GREEN}[{}] Train successfully processed at destination.{RESET}", station_name);
                                }
                                Err(e) => {
                                    println!("{RED}[{}] ERROR processing train at destination: {:?}{RESET}", station_name, e);
                                    let mission_report = MissionReport::Failure(format!("Failed to process Train {} at destination {}: {:?}", train_id, station_name, e));
                                    if reply_to.send(Err(e)).is_err() {
                                        println!("{RED}[{}] DEAD-LETTER: Failed to send failure report for Train {} at destination.{RESET}", station_name, train_id);
                                    } else {
                                        println!("{YELLOW}[{}] Successfully sent failure report for Train {} back to network.{RESET}", station_name, train_id);
                                    }
                                }
                            }
                            
                            println!("{BOLD}{CYAN}[{}] Status Report Requested:{RESET}", station_name);
                            yard.print_report(&roundhouse);
                            println!("{BOLD}{YELLOW}Warehouse Inventory ({}){RESET}", warehouse.inventory.len());
                            for cargo in &warehouse.inventory {
                                println!("  - {} ({}kg)", cargo.item, cargo.actual_weight);
                            }


                            report_to.as_ref().map(|sender| {
                                sender.send(MissionReport::Success(format!("Train {} has arrived at station {}.", train_id, station_name)))
                                    .unwrap_or_else(|e| println!("{RED}[{}] Failed to send arrival report for Train {}: {:?}{RESET}", station_name, train_id, e));
                            });
                      
                        } else {
                            
                            if let Some((distance, route)) = map.find_shortest_path(&station_name, &train.destination) {
                                let next_stop = route.get(1).cloned().unwrap_or_else(|| train.destination.clone()); // The next stop is the second element in the route (index 1), or the destination if the route is just one stop
                                let next_stop_handle = map.get_station_handle(&next_stop).expect("Next stop must exist in the network").clone();
                                let distance_to_next_stop = map.get_distance(&station_name, &next_stop).expect("Distance to next stop must be calculable");
                                let (transit_tx, transit_rx) = mpsc::channel();
                                println!("{BOLD}{GREEN}[{}] Route calculated for Train {}: {}{RESET}", station_name, train.id,  route.join(" -> "));
                                
                                
                                    
                                let train_id = train.id; // Store the train ID for logging inside the thread
                                let my_station_handle = tx.clone(); // Clone the station's own Sender for use in the thread, so we can send SOS if needed

                                match reply_to.send(Ok(())) {
                                    Ok(_) => println!("{YELLOW}[{}] Successfully sent receive confirmation for Train {} back to network.{RESET}", station_name, train_id),
                                    Err(e) => println!("{RED}[{}] DEAD-LETTER: Failed to send receive confirmation for Train {}: {:?}{RESET}", station_name, train_id, e),
                                }

                                let station_name_clone = station_name.clone(); // Clone the station name for use in this thread
                                // This thread will wait for the train to arrive at the next station and then send a command to that station to receive the train
                                thread::spawn(move || {
                                    train.dispatch(distance_to_next_stop).expect("Failed to dispatch train");// This simulates the train traveling to the next station. In a real implementation, you would have more complex logic here to handle the train's movement and interactions with the network.
                                    let speed = train.engine.engine_type.speed();
                                    let time = distance_to_next_stop / (speed as f64); // simulates miles per second
                                    println!("{BOLD}{YELLOW}[{}] Train {} is en route to next stop {}. Estimated time: {:.2} seconds.{RESET}", station_name_clone, train_id, next_stop, time);
                                    thread::sleep(std::time::Duration::from_secs_f64(time)); // Simulate travel time to the next station. In a real implementation, this would be based on distance and train speed.
                                    
                                    
                                    // Using rand, simulate the train crashing with a 10% chance during transit. If it crashes, we issue a Derailment report back to transit_rx and skip the rest of the transit logic. The train is lost, so we don't send it to the next station. However, we return the salvaged TrainCars back to the yard for processing, and we send a MissionReport::Failure back to the mission's reply channel with details of the crash.
                                    let tree_falls = rand::thread_rng().gen_bool(0.1);
                                    if tree_falls {
                                        println!("{RED}🚨 DERAILMENT!{RESET}");

                                        // We send an SOS command BACK to the Station's main mailbox!
                                        // (You will need to pass a clone of the Station's own Sender into the thread)
                                        my_station_handle.send(StationCommand::HandleEmergencySOS {
                                            mission_id: train.mission_id.unwrap_or(0),
                                            surviving_cars: train.cars, // The train dies, but the cars live!
                                            report_to: train.report_to,
                                        }).expect("SOS failed");

                                        return; // Thread ends. Engine drops. The cars are now in limbo until the station processes the SOS and returns them to the yard or purgatory.
                                    } else {
                                        println!("{GREEN}{BOLD}[{}] Train {} has successfully arrived at next stop {}. Sending receive command...{RESET}", station_name_clone, train_id, next_stop);
                                        
                                        
                                        
                                        
                                    





                                    next_stop_handle.send(StationCommand::ReceiveTrain { train, reply_to: transit_tx }).expect("Failed to forward train to next station");
                                    }


                                    match transit_rx.recv() {
                                        Ok(result) => {
                                            println!("{BOLD}{CYAN}[{}] OOOOOOOOH YEAH! Train {} has been received at {}. Finalizing transit...{RESET}", station_name_clone, train_id, next_stop);
                                            // Handle the result of the train being received at the next station if necessary
                                        },
                                        Err(e) => {
                                            println!("{RED}[{}] ERROR receiving transit confirmation for Train {}: {:?}{RESET}", station_name_clone, train_id, e);
                                        }
                                    }
                            
                                    // next_stop_handle.send(StationCommand::ReceiveTrain { train, reply_to }).expect("Failed to forward train to next station");
                                });
                            } else {
                                // THE END OF THE LINE: Emergency Breakdown
                                println!("{BOLD}{RED}[{}] CRITICAL: Track failure! No route to {}. Train {} is stranded!{RESET}", 
                                    station_name, train.destination, train.id);

                                // 1. Mail the bad news back to the Producer so they don't block forever
                                train.report_to.as_ref().map(|sender| {
                                    let _ = sender.send(MissionReport::Failure(
                                        format!("Train {} stranded at {}. Track network failure to {}.", 
                                            train.id, station_name, train.destination)
                                    ));
                                });

                                // 2. Perform an emergency local breakdown
                                println!("{YELLOW}[{}] Initiating emergency breakdown to recover assets...{RESET}", station_name);
                                match yard.disassemble_train(train, &mut roundhouse) {
                                    Ok(payloads) => {
                                        // 3. Store the stranded cargo in the local warehouse
                                        for cargo in payloads {
                                            println!("{YELLOW}[{}] EMERGENCY STORAGE: Stowing {} ({}kg) in local warehouse.{RESET}", 
                                                station_name, cargo.item, cargo.actual_weight);
                                            warehouse.store(cargo);
                                        }
                                    },
                                    Err(e) => println!("{RED}[{}] Disassembly failed during emergency breakdown: {:?}{RESET}", station_name, e),
                                }
                            }
                        }


                    },

                    StationCommand::HandleEmergencySOS { mission_id, surviving_cars, report_to } => {
                        println!("{YELLOW}[{}] Processing emergency SOS. Recovering assets...{RESET}", station_name);
                        // Put the cars back in the yard!
                        let num_cars = surviving_cars.len();
                        for car in surviving_cars {
                            let _ = yard.receive_car(car);
                        }
                        // Mail the failure to the producer!
                        report_to.as_ref().map(|sender| sender.send(MissionReport::Failure(
                            format!("EMERGENCY SOS: Train associated with Mission {} has derailed near station {}. Salvaged {} cars.", 
                                mission_id, station_name, num_cars)
                        )).unwrap_or_else(|e| println!("{RED}[{}] Failed to send SOS report for Mission {}: {:?}{RESET}", station_name, mission_id, e)));
                        println!("{BOLD}{CYAN}[{}] Status Report Requested:{RESET}", station_name);
                        yard.print_report(&roundhouse);
                    },

                    StationCommand::IntakeCar { train_car, reply_to } => {
                        println!("{BOLD}{CYAN}[{}] Received command to intake a new car into the yard.{RESET}", station_name);
                        let result = match yard.receive_car(train_car) {
                            Ok(_) => Ok(()),
                            Err((homeless_car, issues)) => {
                                let reason = format!("Car intake failed due to {:?}", &issues);
                                let homeless_car_id = homeless_car.id;
                                println!("{RED}Yard Error: Failed to intake Car {}: {:?}. Moving to purgatory.{RESET}", homeless_car_id, issues);
                                let rejected_asset = RejectedAsset::new(homeless_car, issues, None);
                                yard.purgatory.push(rejected_asset);
                                Err(TrainError::CarToPurgatory { car_id: homeless_car_id, issues: reason })
                            }
                        };
                        if reply_to.send(result).is_err() {
                            println!("{RED}[{}] DEAD-LETTER: intake-car reply dropped.{RESET}", station_name);
                        }
                    },
                    StationCommand::IntakeEngine { engine, reply_to } => {
                        println!("{BOLD}{CYAN}[{}] Received command to intake a new engine into the roundhouse.{RESET}", station_name);
                        roundhouse.house(engine);
                        if reply_to.send(Ok(())).is_err() {
                            println!("{RED}[{}] DEAD-LETTER: intake-engine reply dropped.{RESET}", station_name);
                        }
                    }
                    StationCommand::PrintStatus => {
                        println!("{BOLD}{CYAN}[{}] Status Report Requested:{RESET}", station_name);
                        yard.print_report(&roundhouse);
                        println!("{BOLD}{YELLOW}Warehouse Inventory ({}){RESET}", warehouse.inventory.len());
                        for cargo in &warehouse.inventory {
                            println!("  - {} ({}kg)", cargo.item, cargo.actual_weight);
                        }
                    },
                    StationCommand::Terminate => {
                        println!("{BOLD}{RED}[{}] Termination command received. Shutting down station thread.{RESET}", station_name);
                        break; // Exit the loop to terminate the thread
                    },
                }

            }
        });
        
    }


}
