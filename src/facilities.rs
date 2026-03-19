use crate::models::{Train, TrainCar, Engine, Mission, TrainError, RejectedAsset, EngineType, Cargo};
use std::collections::{HashMap, VecDeque};

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
    //cargo: Vec<Cargo>,

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


    pub fn receive_car(&mut self, mut car: TrainCar) -> Result<(), (TrainCar, Vec<TrainError>)> {

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
            self.cars.insert(car.id, car);
            Ok(())
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
        let mut issues = Vec::<TrainError>::new();
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

    // pub fn assemble_cars(&mut self, mission: &Mission) -> Result<Vec<TrainCar>, TrainError> {
    //     let car_ids = &mission.required_cars;
    
    //     // Attempt all removals, collecting failures without mutating on partial success
    //     let mut attached_cars = Vec::with_capacity(car_ids.len());
    //     let mut missing = Vec::new();
    
    //     for id in car_ids {
    //         match self.cars.remove(id) {
    //             Some(car) => attached_cars.push(car),
    //             None => missing.push(*id),
    //         }
    //     }
    
    //     if !missing.is_empty() {
    //         // Re-insert any cars we already pulled, rolling back the partial mutation. Let's do it!
    //         for car in attached_cars {
    //             self.cars.insert(car.id, car);
    //         }
    //         // (in a real async system you'd want a transaction, but this is the spirit)
    //         return Err(TrainError::AssemblyFailed { missing_car_ids: missing, engine_returned: 0 });
    //     }
    
    //     Ok(attached_cars)
    // }

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

    pub fn disassemble_train(&mut self, train: Train, roundhouse: &mut Roundhouse) -> Result<Vec<Cargo>, TrainError> {
        let (engine, cars, _id) = (train.engine, train.cars, train.id); // Destructure the "Gestalt"

        // 1. Return the Power
        roundhouse.house(engine);

        // 2. Process the Cars
        let mut returned_cargo = Vec::new();
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
                    if let Some(cargo) = payload {
                        println!("{GREEN}Train {}: Successfully delivered cargo '{}' from Car {} to the yard.{RESET}", _id, cargo.item, car_id_we_just_received);
                        returned_cargo.push(cargo);
                    } else {
                        println!("{YELLOW}Train {}: Car {} had no cargo to unload.{RESET}", _id, car_id_we_just_received);
                    }
                }
            } else {
                // receive_car already handles Purgatory internally in our current code.
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




pub struct Station {
    pub name: String,
    pub yard: Railyard,
    pub warehouse: Warehouse,
    pub roundhouse: Roundhouse,
}



impl Station {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            yard: Railyard::new(),
            warehouse: Warehouse::new(),
            roundhouse: Roundhouse::new(),
        }
    }

    pub fn process_ejected_car(&mut self, train: &mut Train, car_id: u32) {
        if let Some(car) = train.eject_car(car_id) {
            if let Err((homeless_car, issues)) = self.yard.receive_car(car) {
                let rejected_asset = RejectedAsset::new(homeless_car, issues, train.mission_id);
                self.yard.purgatory.push(rejected_asset);
            }
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

        // 4. The Station builds the Gestalt//testing
    
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
                let rejected_asset = RejectedAsset::new(homeless_car, error, None); // We can fill in the timestamp and source_mission later when we implement those features.
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
        if let Ok(payloads) = self.yard.disassemble_train(train, &mut self.roundhouse) {
            for cargo in payloads{
                self.warehouse.store(cargo);
            }
        }
    }
    
    // A helper to inspect the local state
pub fn print_status(&self) {
        println!("\n{BOLD}{CYAN}=== STATION REPORT: {} ==={RESET}", self.name);
        
        // 1. Print Yard & Roundhouse
        self.yard.print_report(&self.roundhouse);

        // 2. Print Warehouse
        println!("{BOLD}{YELLOW}📦 WAREHOUSE INVENTORY ({}) 📦{RESET}", self.warehouse.inventory.len());
        if self.warehouse.inventory.is_empty() {
            println!("    (Warehouse is empty)");
        } else {
            for (i, cargo) in self.warehouse.inventory.iter().enumerate() {
                let contraband_status = if cargo.contraband.is_some() { "[FLAGGED]" } else { "[CLEARED]" };
                println!("    {}. {} ({}kg) {}", i + 1, cargo.item, cargo.actual_weight, contraband_status);
            }
        }
        println!("{BOLD}{CYAN}======================================{RESET}\n");
    }
}
