use crate::models::{Train, TrainCar, Engine, Mission, TrainError, RejectedAsset, EngineType, Cargo};
use std::collections::{HashMap, VecDeque};

use std::f32::consts::E;
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
                    if let Some(engine) = winner_index.and_then(|index| queue.remove(index)) {
                        println!("{GREEN}Roundhouse: Dispatching Engine {} of type {:?} for mission ({}kg over {}km).{RESET}", engine.id, engine.engine_type, total_weight, distance_km);
                        return Some(engine);
                    }
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




// pub struct Station {
//     pub name: String,
//     pub yard: Railyard,
//     pub warehouse: Warehouse,
//     pub roundhouse: Roundhouse,
// }

pub struct Station {
    pub name: String,
    pub tx: Sender<StationCommand>, // The station's command channel for receiving instructions
}

fn assemble_train(
    station_name: &str,
    yard: &mut Railyard,
    roundhouse: &mut Roundhouse,
    mission: &Mission,
    distance: u32,
) -> Result<Train, TrainError> {
    println!("{BOLD}{CYAN}[{}] Orchestrating Assembly for Mission {}...{RESET}", station_name, mission.id);

    let total_weight = yard.get_total_cargo_weight(mission)?;
    let engine = roundhouse
        .find_suitable_engine(total_weight, distance)
        .ok_or(TrainError::NoAvailableEngine)?;
    let attached_cars: Vec<TrainCar> = yard.assemble_cars(mission)?;

    Ok(Train {
        id: yard.generate_new_id(),
        engine,
        cars: attached_cars,
        distance_km: distance,
        mission_id: Some(mission.id),
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
    pub fn new(name: &str) -> Self {
        // Create a channel for this station
        let (tx, rx): (Sender<StationCommand>, Receiver<StationCommand>) = mpsc::channel();

        // instantiate roundhouse, yard, and warehouse, and copy station name, before moving them into the thread
        let mut roundhouse = Roundhouse::new();
        let mut yard = Railyard::new();
        let mut warehouse = Warehouse::new();
        let station_name = String::from(name);
        // Spawn a thread to run the station's internal loop
        thread::spawn(move || {
            // The station's internal state
            println!("{BOLD}{CYAN}[{}] Station is now operational and awaiting commands...{RESET}", station_name);

            // The station's main loop
            for command in rx {
                match command {
                    StationCommand::AssembleMission { mission, distance, reply_to } => {
                        println!("{BOLD}{CYAN}[{}] Received command to assemble mission {}.{RESET}", station_name, mission.id);
                        let result = assemble_train(&station_name, &mut yard, &mut roundhouse, &mission, distance);
                        if let Err(send_error) = reply_to.send(result) {
                            if let Ok(phantom_train) = send_error.0 {
                                if let Err(e) = receive_train_internal(&station_name, &mut yard, &mut roundhouse, &mut warehouse, phantom_train) {//TAKE HEART, TREY! You wrote this. (With help, of course.) You know it works! If we have to process a phantom train, it means the assembly failed after the engine was dispatched, so we have to return that engine to the roundhouse and move any attached cars into purgatory, since they were never officially "yours" to begin with. This is a bit of emergency triage to maintain internal consistency and prevent resource leaks in the system.
                                    println!("{RED}[{}]STATION: ERROR DURING PHANTOM TRAIN PROCESSING: {:?}{RESET}", station_name, e);
                                }
                                else {
                                    println!("{GREEN}[{}]STATION: Successfully processed phantom train for failed assembly of mission {}.{RESET}", station_name, mission.id);
                                }
                            }
                            println!(
                                "{RED}[{}] DEAD-LETTER: assemble reply dropped (mission_id={}, request_id={}).{RESET}",
                                station_name, mission.id, mission.request_id
                            );
                        } else {
                            println!("{GREEN}[{}]STATION: Successfully assembled train for mission {}. Reply sent to network.{RESET}", station_name, mission.id);
                        }
                    },
                    StationCommand::ReceiveTrain { train, reply_to } => {
                        let result = receive_train_internal(
                            &station_name,
                            &mut yard,
                            &mut roundhouse,
                            &mut warehouse,
                            train,
                        );
                        if reply_to.send(result).is_err() {
                            println!("{RED}[{}] DEAD-LETTER: receive-train reply dropped.{RESET}", station_name);
                        }
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
        // The thread will run an infinite loop, waiting for commands to arrive on the rx channel, and processing them as they come in. This is where the station's internal logic will live, and it will have access to its own yard, roundhouse, and warehouse to manage its operations.
        Self {
            name: String::from(name),
            tx, // We return the sending end to the main thread so it can send commands to this station
        }
    }


}
