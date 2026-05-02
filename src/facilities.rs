use crate::models::{Train, TrainCar, Engine, Mission, TrainError, RejectedAsset, EngineType, Cargo, FreightOrder ,Location, MissionReport};
use crate::network::{GlobalLedger, RailwayNetwork};
use core::error;
use std::collections::{HashMap, HashSet, VecDeque};
use rand::Rng;

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;
use crate::models::StationCommand;

use std::sync::atomic::{AtomicU32, Ordering};

// (Don't forget to paste your color constants here too, or put them in a shared module later)
const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";

const EMPTY_CAR_WEIGHT: u32 = 2000; // Let's say every empty car weighs 2000kg. This is important for fuel calculations, because the engine has to pull not just the cargo, but also the weight of the cars themselves.

// We start at 100 so it doesn't collide with the hardcoded cars (1-6) you made in main!
static GLOBAL_CAR_ID: AtomicU32 = AtomicU32::new(1000);

static GLOBAL_ORDER_ID: AtomicU32 = AtomicU32::new(1000);

static GLOBAL_REQUEST_ID: AtomicU32 = AtomicU32::new(0000);

//static GLOBAL_MISSION_ID: AtomicU32 = AtomicU32::new(0000);

static GLOBAL_TRAIN_ID: AtomicU32 = AtomicU32::new(0000);


pub enum GossipStrategy {
    Flood,
    Swarm,
}


pub trait CanReport {
    // Every struct that signs this must tell us its name
    fn get_reporter_name(&self) -> &str;

    // DEFAULT BEHAVIOR: Free code!
    fn send_failure_report(&self, mission_id: u32, reason: &str, channel: Option<Sender<MissionReport>>) {
        let name = self.get_reporter_name();
        let message = format!("Mission {} failed at {}. Reason: {}", mission_id, name, reason);
        if let Some(chan) = channel {
            let _ = chan.send(MissionReport::Failure(message));
        } else {
            println!("{RED}[{}] DEAD-LETTER: No reply channel available to report failure for mission {}. Reason: {}{RESET}", name, mission_id, reason);
        }
    }

    fn send_partial_failure_report(&self, mission_id: u32, reason: &str, lost_cargo_ids: &[u32], channel: Option<Sender<MissionReport>>) {
        let name = self.get_reporter_name();
        let message = format!("Mission {} partially failed at {}. Reason: {}. Lost car IDs: {:?}", mission_id, name, reason, lost_cargo_ids);
        if let Some(chan) = channel {
            let _ = chan.send(MissionReport::PartialFailure(message));
        } else {
            println!("{RED}[{}] DEAD-LETTER: No reply channel available to report partial failure for mission {}. Reason: {}. Lost car IDs: {:?}{RESET}", name, mission_id, reason, lost_cargo_ids);
        }
    }

    fn send_success_report(&self, mission_id: u32, details: &str, channel: Option<Sender<MissionReport>>) {
        let name = self.get_reporter_name();
        let message = format!("Mission {} successful at {}. Details: {}", mission_id, name, details);
        
        
        if let Some(chan) = channel {
            let _ = chan.send(MissionReport::Success(message));
        } else {
            println!("{RED}[{}] DEAD-LETTER: No reply channel available to report success for mission {}. Details: {}{RESET}", name, mission_id, details);
        }

    }
}



pub trait Receivable {
    fn get_payload(&mut self) -> Vec<Cargo>;

// The Trait Bound is the `<T: Receivable>` part!
fn handle_arrival<T: Receivable>(&mut self, mut vehicle: T) {
    let cargo = vehicle.get_payload(); 
    // The compiler allows this because the Trait guarantees the method exists!
}
}



pub trait TransitVehicle {
    // 1. For logging:
    fn get_id(&self) -> u32;
    
    // 2. For the Dijkstra Map check:
    fn get_destination(&self) -> &String;
    
    // 3. For reporting to the Producer:
    fn get_mission_id(&self) -> Option<u32>;
    fn get_report_channel(&self) -> Option<Sender<MissionReport>>;
    
    // 4. To fulfill the delivery (whether it's breaking down cars or dropping a drone payload)
    fn deliver_payload(self) -> Vec<Cargo>; 
}


















pub struct Railyard {
    pub id: u32,
    pub trains: Vec<Train>,
    pub cars: HashMap<u32, TrainCar>,
    pub next_train_id: u32,
    pub purgatory: Vec<RejectedAsset>,
}


impl Railyard {
    




    /// Finds any empty car in the yard, loads the cargo into it, and returns the car.
    pub fn load_cargo_into_empty_car(&mut self, cargo: Cargo) -> Result<TrainCar, TrainError> {
        // 1. Find the ID of an empty car
        let empty_car_id = self.cars.iter()
            .find(|(_, car)| car.cargo.is_none())
            .map(|(&id, _)| id); // Just grab the ID

        // 2. If we found one, remove it from the yard, load it, and return it
        if let Some(id) = empty_car_id {
            let mut car = self.cars.remove(&id).unwrap();
            car.cargo = Some(cargo); // LOAD THE CAR!
            Ok(car)
        } else {
            Err(TrainError::MissionImpossible { 
                reason: "No empty cars available in the yard!".to_string() 
            })
        }
    }


    pub fn load_cargo_into_empty_cars (&mut self, cargo: Vec<Cargo>) -> Result<Vec<TrainCar>, TrainError> {
        let mut cars = Vec::<TrainCar>::new();
        for cargo_item in cargo {
            let car = self.load_cargo_into_empty_car(cargo_item)?;
            cars.push(car);
        }
        Ok(cars)
    }



    // Copilot was here! Helping with the functional stuff. Thanks, buddy!
    pub fn validate_empty_cars(&self, mission: &Mission) -> bool {
        return mission.cargo_ids.len() <= self.cars.values().filter(|car| car.cargo.is_none()).count();
    }






    fn new(id: u32) -> Self {
        Railyard {
            id,
            trains: Vec::new(),
            cars: HashMap::new(),
            next_train_id: 1,
            purgatory: Vec::new(),
        }
    }

    fn generate_new_train_id(&mut self) -> u32 {
        GLOBAL_TRAIN_ID.fetch_add(1, Ordering::SeqCst)
    }
    fn generate_new_request_id(&mut self) -> u32 {
        GLOBAL_REQUEST_ID.fetch_add(1, Ordering::SeqCst)
    }
    
    fn generate_new_car_id(&self) -> u32 {
        GLOBAL_CAR_ID.fetch_add(1, Ordering::SeqCst)
    }

    // fn generate_new_mission_id(&self) -> u32 {
    //     GLOBAL_MISSION_ID.fetch_add(1, Ordering::SeqCst)
    // }

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

    // This essential method returns a vector of fully loaded cars ready to be hitched up to the engine and sent on its way to fulfill the mission.
    pub fn assemble_cars(&mut self, cargo: Vec<Cargo>) -> Result<Vec<TrainCar>, TrainError> {
        
        // We've already validated that we have enough empty cars for this mission before we even started assembling, so we can be confident that this won't fail due to lack of cars. If it does fail, it's an unexpected error that we should know about immediately, which is why we use the `?` operator to propagate any errors up to the caller without having to write explicit error handling logic here.
        let mut cars = Vec::<TrainCar>::new();
        for cargo_item in cargo {
            let car = self.load_cargo_into_empty_car(cargo_item)?;
            cars.push(car);
        }
        Ok(cars)
        
    }


}


pub struct Roundhouse {
    pub id: u32,
    pub stalls: HashMap<EngineType, VecDeque<Engine>>,
}



impl Roundhouse {
    pub fn new(id:u32) -> Self {
        Roundhouse {
            id,
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


    pub fn check_suitable_engine(&self, total_weight: f64, distance_km: f64) -> Option<&Engine> {
        // We check each stall in order of engine strength
        let roster = [
            EngineType::Percy, 
            EngineType::Thomas, 
            EngineType::Diesel, 
            EngineType::Gordon
        ];

        for etype in roster {
            if let Some(queue) = self.stalls.get(&etype) {
                for engine in queue {
                    if engine.can_complete_mission(total_weight, distance_km) {
                        return Some(engine);
                    }
                }
            }
        }
        None // If we loop through the whole roster and find nothing, return None.
    }

    // Create a method that tells us only if we possess an engine perfect in strenght, not too weak, not too strong, just right. This is for the "Goldilocks" gossip strategy where we only want to ask for help if we don't have the perfect engine in our own roundhouse.
    pub fn check_ideal_engine(&self, total_weight: f64, distance_km: f64) -> Option<&Engine> {
        // We check each stall in order of engine strength
        let roster = [
            EngineType::Percy, 
            EngineType::Thomas, 
            EngineType::Diesel, 
            EngineType::Gordon
        ];

        for etype in roster {
            if let Some(queue) = self.stalls.get(&etype) {
                for engine in queue {
                    if engine.can_complete_mission(total_weight, distance_km) {
                        // Check if it's a perfect match (not too weak, not too strong)
                        if engine.is_ideal_for_mission(total_weight) {
                            return Some(engine);
                        }
                    }
                }
            }
        }
        None // If we loop through the whole roster and find nothing, return None.
    }

    pub fn find_suitable_engine(&mut self, total_weight: f64, distance_km: f64) -> Result<Engine, TrainError> {
        
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
                println!("{YELLOW}Roundhouse {}: Checking for available {:?} engines...{RESET}", self.id, etype);
                
                // If it is, look inside that specific stall
                if let Some(queue) = self.stalls.get_mut(&etype) {
                    
                    // 1. Find the position of the first capable engine
                    let winner_index = queue.iter().position(|engine| {
                        engine.can_complete_mission(total_weight, distance_km)
                    });

                    // 2. Chain it using the `.and_then()` you love!
                    // If position returned Some(index), and_then passes that index into queue.remove()
                    if let Some(engine) = winner_index.and_then(|index| queue.remove(index)) {
                        println!("{GREEN}Roundhouse {}: Dispatching Engine {} of type {:?} for mission ({}kg over {}km).{RESET}", self.id, engine.id, engine.engine_type, total_weight, distance_km);
                        return Ok(engine);
                    }
                }

            }
        }
        
        // If we loop through the whole roster and find nothing, return an error.
        println!("{RED}Roundhouse {}: No suitable engines available for mission ({}kg over {}km).{RESET}", self.id, total_weight, distance_km);
        Err(TrainError::MissionImpossible { reason: "NO ENGINES CAN COMPLETE MISSION!".to_string() })
    }
}


pub struct Warehouse {
    pub id: u32,
    pub inventory: HashMap<u32, Cargo>, // We can use the cargo ID as the key for easy retrieval and inventory management
}

impl Warehouse {
    pub fn new(id:u32) -> Self {
        Warehouse {
            id,
            inventory: HashMap::new(),
        }
    }

    pub fn store(&mut self, cargo: Cargo) {
        println!("{BOLD}{YELLOW}Warehouse: Received {} ({}kg) for processing/holding.{RESET}", cargo.item, cargo.actual_weight);
        self.inventory.insert(cargo.id, cargo);
    }

    pub fn process_outbound(&mut self) {
        // This represents fulfillment to the "outside world"
        let fulfilled = self.inventory.len();
        self.inventory.clear();
        if fulfilled > 0 {
            println!("{BOLD}{GREEN}Warehouse: Successfully processed and delivered {} cargo shipments to the outside world.{RESET}", fulfilled);
        }
    }

    
    pub fn get_total_cargo_weight(&self, mission: &Mission) -> Result<u32, TrainError> {


        // We extract the data we need from the mission
        let cargo_ids = &mission.cargo_ids;
        let mut missing_ids = Vec::new(); // Create a ledger for failures
        let mut total_weight = 0;

        for id in cargo_ids {
            match self.inventory.get(id) {
                Some(cargo) => total_weight += cargo.actual_weight,// + EMPTY_CAR_WEIGHT, // Add the weight of the cargo AND the car it's in, because the engine has to pull both!
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


    pub fn get_cargo_by_id(&mut self, id: u32) -> Result<Cargo, TrainError> {
        match self.inventory.remove(&id) {
            Some(cargo) => Ok(cargo),
            None => Err(TrainError::MissingCargo { cargo_id: vec![id] }),
        }
    }


    pub fn get_cargo_by_ids(&mut self, ids: &[u32]) -> Result<Vec<Cargo>, TrainError> {
        let mut cargo = Vec::new();
        let mut missing_ids = Vec::new();

        for id in ids {
            match self.get_cargo_by_id(*id) {
                Ok(cargo_item) => cargo.push(cargo_item),
                Err(_) => missing_ids.push(*id),
            }
        }

        if missing_ids.is_empty() {
            Ok(cargo)
        } else {
            for cargo_item in cargo {
                self.store(cargo_item); // Rollback any cargo we successfully retrieved to save it from Rust's ownership wrath
            }
            Err(TrainError::MissingCargo { cargo_id: missing_ids }) // Return all missing IDs
        }
    }

}
















// This is JUST data. No threads, no channels, no logic.
pub struct StationMetadata {
    pub id: u32,
    pub name: String,
    pub location: Location,

}

    pub struct Station {
        // pub id: u32,
        // pub name: String,
        // pub neighbors: HashMap<u32, Sender<StationCommand>>, // The station's direct neighbors and their command channels
        // pub tx: Sender<StationCommand>, // The station's command channel for receiving instructions
        // pub map: Arc<RailwayNetwork>, // The shared network map for the station to access
        // pub location: Location, // The station's location on the network (for distance calculations)
        // We no longer hold the yard, warehouse, and roundhouse directly in the Station struct
    }   


impl Station {
    pub fn new(id: u32, name: &str, neighbors: HashMap<u32, Sender<StationCommand>>, tx: Sender<StationCommand>, map: Arc<RailwayNetwork>, ledger: Arc<Mutex<GlobalLedger>>, rx: Receiver<StationCommand>) {
        // Create a channel for this station
        // instantiate roundhouse, yard, and warehouse, and copy station name, before moving them into the thread
        let station_name = String::from(name);
        let station_id = id;
        let tx = tx; // The station's own Sender for receiving commands

        let mut state = StationState::new(id, station_name.clone(), neighbors, map.clone(), ledger.clone(), tx.clone());
        // Spawn a thread to run the station's internal loop
        thread::spawn(move || {
            // The station's internal state
            println!("{BOLD}{CYAN}[{}]::Station {} is now operational and awaiting commands...{RESET}", station_name, station_id);

            // The station's main loop
            for command in rx {
                match command {
                    StationCommand::AssembleMission { mission} => {
                        state.handle_assemble_mission(mission);
                    },
                    StationCommand::ReceiveTrain {mut train, reply_to } => {
                        state.handle_receive_train(train, reply_to);
                    },

                    StationCommand::HandleEmergencySOS { mission_id, destination, surviving_cars, report_to } => {
                        state.handle_emergency_sos(mission_id, destination, surviving_cars, report_to);
                    },

                    StationCommand::IntakeCar { cars, reply_to } => {
                       state.handle_intake_cars(cars, Some(reply_to));
                    },
                    StationCommand::IntakeCargo { cargo, reply_to } => {
                        state.handle_intake_cargo(cargo, Some(reply_to));
                    },
                    StationCommand::IntakeEngine { engine, reply_to } => {
                        println!("{BOLD}{CYAN}[{}] Received command to intake a new engine into the roundhouse.{RESET}", station_name);
                        state.handle_intake_engine(engine, Some(reply_to));
                    }
                    StationCommand::NewNeighbor { neighbor, neighbor_tx } => {
                        state.handle_new_neighbor(neighbor, neighbor_tx);
                    },
                    StationCommand::RequestEmptyCars { count } => {
                        state.handle_request_empty_cars(count);
                    }
                    StationCommand::EngineRequest { requester_id, request_id, mission_id, min_capacity, mission_max_hop, ttl, branch_notified, notified_count } => {
                        state.handle_engine_request(requester_id, request_id, mission_id, min_capacity, mission_max_hop, ttl, branch_notified, notified_count);
                    }
                    StationCommand::EngineRequestResponse { request_id, station_id, engine } => {
                        //TODO: We need to know which mission this is for so we can route the engine to the right place once we get it. We can add that to the command if needed.
                    }
                    StationCommand::CheckStatus => {// The Alarm Clock: station sends to itself every X seconds to trigger regular status checks and maintenance tasks like checking pending missions, gossiping about engines, etc.
                        println!("{BOLD}{CYAN}[{}]::Station {}: Checking pending missions...{RESET}", station_name, station_id);
                        state.check_pending_missions();
                    }
                    StationCommand::PrintStatus => {
                        println!("{BOLD}{CYAN}[{}]::Station {}: Status Report Requested:{RESET}", station_name, station_id);
                        state.print_status();
                    },
                    StationCommand::Terminate => {
                        println!("{BOLD}{RED}[{}]::Station {}: Termination command received. Shutting down station thread.{RESET}", station_name, station_id);
                        break; // Exit the loop to terminate the thread
                    },
                }

            }
        });
        
    }


}




pub struct StationState {
    pub id: u32,
    pub name: String,
    pub yard: Railyard,
    pub roundhouse: Roundhouse,
    pub warehouse: Warehouse,
    pub neighbors: HashMap<u32, Sender<StationCommand>>,
    pub map: Arc<RailwayNetwork>,
    pub ledger: Arc<Mutex<GlobalLedger>>,
    pub seen_engine_request: HashSet<u32>, // To prevent engine request loops, we keep track of which engine requests we've already seen and handled. The key is the mission ID. When we receive an engine request, we check this HashSet first. If we've already seen it, we ignore it to prevent infinite loops of stations passing the same request back and forth. If we haven't seen it, we mark it as seen and proceed with handling the request.
    pub tx: Sender<StationCommand>, // The Boomerang
    pub pending_missions: Vec<Mission>, // 
}




impl CanReport for StationState {
    fn get_reporter_name(&self) -> &str {
        &self.name
    }
}


impl StationState {
    pub fn new(id: u32, name: String, neighbors: HashMap<u32, Sender<StationCommand>>, map: Arc<RailwayNetwork>, ledger: Arc<Mutex<GlobalLedger>>, tx: Sender<StationCommand>) -> Self {
        StationState {
            id,
            name,
            yard: Railyard::new(id),
            roundhouse: Roundhouse::new(id),
            warehouse: Warehouse::new(id),
            neighbors,
            map,
            ledger,
            tx,
            seen_engine_request: HashSet::new(),
            pending_missions: Vec::new(),
        }
    }


    // The VIP Pass is `&mut self`. This allows the method to open its own briefcase!
    pub fn handle_assemble_mission(
        &mut self, 
        mission: Mission, 
        //distance: f64, 
        //route: Vec<String>, 
        //destination: String, 
        //reply_to: Sender<Result<Train, TrainError>>
    ) {
        println!("{BOLD}{CYAN}[{}] Received command to assemble mission {}.{RESET}", self.name, mission.id);
        
        // 1. Paste your assemble_train logic here!
        // First, we need to use our map! Choo choo!



        let (distance, route) = match self.map.find_shortest_path(self.id, mission.destination) {
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
              
                match mission.reply_channel {
                    Some(sender) => {
                        let report = MissionReport::Failure(format!(
                            "Mission {} failed during assembly: Destination {} is unreachable from {}.",
                            mission.id, mission.destination, self.name
                        ));
                        let _ = sender.send(report);
                    },
                    None => {
                        println!("{RED}[{}] DEAD-LETTER: No reply channel available to report assembly failure for mission {} due to unreachable destination.{RESET}", self.name, mission.id);
                    }
                }
                return;
              
                // if reply_to.send(Err(error)).is_err() {
                //     println!("{RED}[{}] DEAD-LETTER: Failed to send assembly failure for mission {} due to unreachable destination.{RESET}", self.name, mission.id);
                // }
                // return;
            }
        };

        // At this point, we have the distance and route calculated, so we can proceed with the assembly logic using these values.

        // Now for the fun part: we're going to completely rewrite assemble_train as part of the Station's responsibilities, because the Station is now the mastermind behind the whole operation, and it needs to have access to its internal state (the yard and roundhouse) to pull this off. The network is just a map and dispatcher, so it makes more sense for the Station to handle the assembly logic directly.

        println!("{BOLD}{CYAN}[{}]::Station {}: Starting assembly for Mission {}: {}kg to {} via {:?}.{RESET}", self.name, self.id, mission.id, mission.cargo_ids.len(), mission.destination, route);
        // The first thing we need to do is figure out the total weight of the cargo, because that will determine which engines we can use. 
        let total_cargo_weight = match self.warehouse.get_total_cargo_weight(&mission) {
            Ok(weight) => {
                println!("{YELLOW}Warehouse: Total cargo weight for Mission {} is {}kg (including empty car weight).{RESET}", mission.id, weight);
                weight
            },
            Err(e) => {
                println!("{RED}Yard Error: Failed to calculate total cargo weight for Mission {}: {:?}.{RESET}", mission.id, e);
                
                let details = "Failed to calculate total cargo weight. This likely means that one or more cargo items specified in the mission's cargo_ids are missing from the warehouse inventory. The warehouse is responsible for keeping track of all cargo and their weights, so if it cannot provide the total weight, it indicates a critical issue with the inventory management. This failure prevents us from determining whether we have a suitable engine available in the roundhouse, which is essential for proceeding with the assembly of the train. Please investigate the warehouse inventory and ensure that all cargo items for this mission are properly stored and accounted for.";
                self.report_mission_failure(&mission, details);
                //self.report_mission_failure(&mission, &format!("Failed to calculate total cargo weight: {:?}", e));
                return;
            }
        };

        // 2. Mathematically project the final gross weight of the train!
        let num_cars_needed = mission.cargo_ids.len() as u32;
        let empty_cars_weight = num_cars_needed * 2000;
        let true_total_weight = total_cargo_weight + empty_cars_weight;

        println!("{YELLOW}Warehouse: Total projected gross weight for Mission {} is {}kg ({}kg cargo + {}kg rolling stock).{RESET}", 
            mission.id, true_total_weight, total_cargo_weight, empty_cars_weight);

        // Before we even try to find an engine, let's check if we have enough empty cars in the yard to load all the cargo. 
        match self.yard.validate_empty_cars(&mission) {
            true => println!("{GREEN}Yard: Validation successful for Mission {}. Enough empty cars available.{RESET}", mission.id),
            false => {
                println!("{RED}Yard Error: Validation failed for Mission {}. Not enough empty cars available.{RESET}", mission.id);
                //let error = TrainError::MissionImpossible { reason: "Not enough empty cars available".to_string() };
                let details = "Not enough empty cars available for the mission. This indicates a shortage in the yard's inventory of empty cars, which is critical for assembling the train. Please investigate the yard's inventory and ensure that sufficient empty cars are available for upcoming missions.";

                
                // self.tx.send(StationCommand::RequestEmptyCars { count: mission.cargo_ids.len() as u32 }).unwrap_or_else(|e| {
                //     println!("{RED}[{}] DEAD-LETTER: Failed to send request for empty cars for mission {} due to yard validation failure.{RESET}", self.name, mission.id);
                // });
                match self.tx.send(StationCommand::RequestEmptyCars { count: mission.cargo_ids.len() as u32 }) {
                    Ok(_) => println!("{YELLOW}[{}]::Station {}: Sent request for {} empty cars to yard due to validation failure for Mission {}.{RESET}", self.name, self.id, mission.cargo_ids.len(), mission.id),
                    Err(e) => println!("{RED}[{}] DEAD-LETTER: Failed to send request for empty cars for mission {} due to yard validation failure. Error: {:?}{RESET}", self.name, mission.id, e),
                }



                self.report_mission_failure(&mission, details);
                return;
            }
        }

        // Knowing max hop distance is crucial for the engine selection. It ensures the engine's fuel capacity can handle the longest stretch of track without refueling.
        let mut max_hop_distance = 0.0;
        for i in 0..route.len() - 1 {
            if let Some(dist) = self.map.get_distance(route[i], route[i+1]) {
                if dist > max_hop_distance {
                    max_hop_distance = dist;
                }
            }
        }

        
        //let strict_mode = true; // Default is strictly "Goldilocks" - we will only accept an engine that is a perfect match for the mission's needs. If we don't have the perfect engine, we ask for help instead of just taking the next best thing. If pending missions becomes backlogged, we can consider relaxing this to allow for "overqualified" engines to take on missions that are below their ideal capacity, but for now we want to focus on the Goldilocks strategy to really test the engine request and network collaboration features.

        // Now we can finally check the roundhouse for a suitable engine, using the total weight and distance to determine which engines are capable of fulfilling this mission.
        let engine = match self.roundhouse.find_suitable_engine(true_total_weight as f64, max_hop_distance) { // We use the max_hop_distance for the engine check because that's the longest single stretch of track the engine's fuel needs to cover
            Ok(engine) => engine,
            Err(e) => {
                println!("{RED}Roundhouse {} Error: Failed to find suitable engine for Mission {}: {:?}.{RESET}", self.id, mission.id, e);
                
                let mission_id = mission.id;
                let request_id = self.yard.generate_new_request_id(); // We can use the yard's ID generator to create unique request IDs for tracking engine requests across the network.
                
                // let details = "No suitable engines available for the mission. This indicates that the roundhouse does not have any engines that are capable of handling the required total weight and distance for this mission. Please investigate the roundhouse inventory and ensure that sufficient engines are available and properly maintained for upcoming missions.";
                // self.report_mission_failure(&mission, details);
                self.pending_missions.push(mission);

                self.initiate_engine_request(self.id, request_id, Some(mission_id), true_total_weight as f64, max_hop_distance, 8); // We can set a TTL of 8 to allow the request to propagate through the network without risking infinite loops. This gives enough time for neighboring stations to check their roundhouses and respond if they have a suitable engine, while also ensuring that the request doesn't bounce around indefinitely if no suitable engines are available in the network.

                // for neighbor in self.neighbors.values() {
                //     match neighbor.send(StationCommand::EngineRequest { 
                //         requester_id: self.id, 
                        
                //         min_capacity: true_total_weight as f64, 
                //         mission_max_hop: max_hop_distance, 
                //         ttl: 3 // We can set a TTL to prevent infinite loops of requests between stations
                //         b
                //     }) {
                //         Ok(_) => println!("{YELLOW}[{}] Sent engine request to neighbor due to lack of suitable engines for Mission {}.{RESET}", self.name, mission.id),
                //         Err(e) => println!("{RED}[{}] DEAD-LETTER: Failed to send engine request to neighbor for mission {}. Error: {:?}{RESET}", self.name, mission.id, e),
                //     }
                // }


                return;
            }
        };

        // If we made it this far, it means we have a suitable engine and enough empty cars, so now we can actually pull the cargo out of the warehouse and continue with the assembly process.
        let cargo: Vec<Cargo> = match self.warehouse.get_cargo_by_ids(&mission.cargo_ids) {
            Ok(cargo) => cargo,
            Err(e) => {
                println!("{RED}Warehouse Error: Failed to retrieve cargo for Mission {}: {:?}.{RESET}", mission.id, e);
                let details = "Failed to retrieve cargo for the mission. This likely means that one or more cargo items specified in the mission's cargo_ids are missing from the warehouse inventory, which is critical for fulfilling the mission's objectives. Please investigate the warehouse inventory and ensure that all cargo items for this mission are properly stored and accounted for.";
                self.report_mission_failure(&mission, details);
                return;
            }
        };

        // At this point, we have the engine, the cargo, and we know we have enough empty cars, so we can proceed to load the cargo into the cars and attach them to the engine.
        let attached_cars = match self.yard.assemble_cars(cargo) {
            Ok(cars) => cars,
            Err(e) => {
                println!("{RED}Yard Error: Failed to assemble cars for Mission {}: {:?}.{RESET}", mission.id, e);
                // Since we already took the engine out of the roundhouse, we need to return it back to avoid losing it due to a failed assembly!
                self.roundhouse.house(engine);
                // if reply_to.send(Err(e)).is_err() {
                //     println!("{RED}[{}] DEAD-LETTER: Failed to send assembly failure for mission {} due to car assembly error.{RESET}", self.name, mission.id);
                // }
                
                
                // match mission.reply_channel {
                //     Some(sender) => {
                //         let report = MissionReport::Failure(format!(
                //             "Mission {} failed during assembly: {:?}",
                //             mission.id, e
                //         ));
                //         let _ = sender.send(report);
                //     },
                //     None => {
                //         println!("{RED}[{}] DEAD-LETTER: No reply channel available to report assembly failure for mission {} due to car assembly error.{RESET}", self.name, mission.id);
                //     }
                // }
                let details = "Failed to assemble cars for the mission. This indicates that there was an issue during the process of preparing the train cars for departure, which is critical for the successful execution of the mission. Please investigate the yard's assembly process and ensure that all necessary resources and procedures are in place for upcoming missions.";
                self.report_mission_failure(&mission, details);

                return;
            }
        };



        let train = Train {
            id: self.yard.generate_new_train_id(),
            engine,
            cars: attached_cars,
            mission_id: Some(mission.id), // We can include the whole mission in the train for easy access to all its details during transit and at the destination, which will be helpful for reporting and any potential issues that arise during the journey.
            destination: mission.destination.clone(),
            report_to: mission.reply_channel.clone(),
        };

        self.dispatch_train(train, route);
    }

    
pub fn retry_pending_missions(&mut self) {
    // 1. Drain the HashMap into a local Vector. 
    // The moment this line finishes, `self.pending_missions` is empty and un-borrowed!
    let parked_missions: VecDeque<Mission> = self.pending_missions.drain(..).collect();

    // 2. Feed them right back into the funnel!
    for mission in parked_missions {
        println!("{YELLOW}[{}]{RESET} Retrying parked mission {}...", self.name, mission.id);
        self.handle_assemble_mission(mission);
    }
}


    pub fn handle_receive_train(&mut self, mut train: Train, reply_to: Sender<Result<(), TrainError>>) {
        let _ = reply_to.send(Ok(())); // Send success back to transit thread so it can terminate.
        train.engine.refuel(); // Refuel the engine upon arrival to ensure it's ready for the next leg of the journey or for disassembly if this is the final destination.
        //println!("{:?}", train);
        println!("{GREEN}[{}]::Station {}: Processing arrival of Train {}.{RESET}", self.name, self.id, train.id);
        let final_destination = train.destination;
        let current_location = self.id;
        let station_tx_clone = self.tx.clone(); // Clone the station's own Sender for use in this method, so we can send SOS if needed

        if current_location == final_destination {
            // TODO: Check to see if the train only has an engine and no cars. It's an engine_request response. We need to notify . . . who exactly, Polaris?
            println!("{GREEN}[{}]::Station {}: Train {} has reached its final destination! Unloading...{RESET}", self.name, self.id, train.id);
            //crack the egg
            let ( engine, cars, mission_id, report_to) = (train.engine,train.cars, train.mission_id, train.report_to); // Destructure the "Gestalt"
            //let num_cars = cars.len();
            //let failed_ids: Vec<u32>; // We can fill this with any issues that arise during disassembly, and then include it in the MissionReport for transparency and debugging. For now, we'll just keep it empty to represent a perfect disassembly.
            // 1. Return the Power
            //engine.refuel(); // We can refuel the engine before returning it to the roundhouse
            self.roundhouse.house(engine);
            // 2. Return the Cars
            let failed_ids = self.process_cars(cars, mission_id); // We can extract this logic into a separate method to keep things cleaner, and it can return the ledger of any failed cars for reporting.

            if failed_ids.is_empty() {
                
                let details = "Successfully disassembled train and processed all cargo without issues.";
                self.send_success_report(mission_id.expect("Unloading cargo after receiving a train implies a Mission is involved! There should be an id!"), details, report_to);
            } else {
                let details = "Partial success during train disassembly. Some items in purgatory.";
                 self.send_partial_failure_report(mission_id.expect("Unloading cargo after receiving a train implies a Mission is involved! There should be an id!"), details, &failed_ids, report_to);
            }

            self.print_status();

            



            // let still_pending:Vec<Mission> = self.pending_missions.drain().collect(); // We can drain the pending missions and attempt to retry them now that we've freed up the engine and cars from this completed mission. This is a simple way to implement a retry mechanism for missions that were waiting on resources that are now available. We can also add some logic to prevent infinite retries or to prioritize certain missions if needed.
            // for mission in still_pending {
            //     println!("{YELLOW}[{}]::Station {}: Retrying pending Mission {} after successful completion of Train {}.{RESET}", self.name, self.id, mission.id, id);

            // }

            self.retry_pending_missions();









        } else {
            //train.engine.refuel(); // Refuel the engine upon arrival to ensure it's ready for the next leg of the journey

            let mission_id = train.mission_id;
            let final_destination = train.destination;
            let current_location = self.id;
            let (distance, route) = match self.map.find_shortest_path(current_location, final_destination) {
                Some((d, r)) => {
                    println!(
                        "{YELLOW}Network: Shortest path for Mission {} Train {} to final destination {} is {} km via {:?}.{RESET}",
                        mission_id.unwrap_or(0), train.id, final_destination, d, r
                    );
                    (d, r)
                },
                None => {
                    println!("{RED}Network Error: No track laid between {} and {}. Cannot forward train.{RESET}", self.name, final_destination);
                    // --- THE VOID PATCH: Salvage Operation ---
                    self.roundhouse.house(train.engine);
                    for car in train.cars {
                        let car_id = car.id;
                        match self.yard.receive_car(car) {
                            Ok(Some(cargo)) => { self.warehouse.store(cargo); },
                            Ok(None) => {}, // Car is empty but safely in the yard
                            Err((homeless_car, e)) => {
                                println!("{RED}Train {}: Failed to process Car {} during salvage: {:?}. Moving to purgatory.{RESET}", train.id, car_id, e);
                                let rejected_asset = RejectedAsset::new(homeless_car, e, train.mission_id,);
                                self.yard.purgatory.push(rejected_asset);
                            }
                        }
                    }
                    // ----------------------------------------
                    let error = TrainError::MissionImpossible { reason: "Destination unreachable".to_string() };
                    if reply_to.send(Err(error)).is_err() {
                        println!("{RED}[{}] DEAD-LETTER: Failed to send transit failure for Train {} due to unreachable destination.{RESET}", self.name, train.id);
                    }
                    // if let Some(sender) = train.report_to {
                    //     let reason = "Failed at the None arm of the Dijkstra check";
                    //     self.send_failure_report(mission_id, reason, &sender);
                    //     // let report = MissionReport::Failure(format!(
                    //     //     "Train {} failed to reach final destination {} because it is unreachable from {}.",
                    //     //     train.id, final_destination, self.name
                    //     // ));
                    //     // let _ = sender.send(report);
                    // }
                    let reason = "Failed at the None arm of the Dijkstra check";
                    self.send_failure_report(train.mission_id.expect("This is a failure report; There should be a mission_id on this train!"), reason, train.report_to);
                    return;
                }
            };
            self.dispatch_train(train, route);               
        }
    }

    // This is the method we call when a train arrives with an SOS from a failed mission. The engine is lost, but some or all of the cars survive and make it to the station. We need to process those cars, report on the situation, and then dispatch a replacement train to fulfill the original mission if possible.
    // Destination is critical for this method, because the original mission's destination may now be unreachable due to the emergency, so we need to update the mission with a new destination (this station) for the replacement train, and then rely on the network's routing logic to find a new path from this station to the original destination that avoids whatever caused the emergency in the first place.
    pub fn handle_emergency_sos(&mut self, mission_id: u32, destination: u32, surviving_cars: Vec<TrainCar>, report_to: Option<Sender<MissionReport>>) {
        println!("{RED}[{}] 🚨 EMERGENCY: Processing SOS for Mission {}.{RESET}", self.name, mission_id);
        
        // We'll need the surviving cargo ids to create the replacement freight order. This ensures that they can be accessed by the producer of the replacement train, so they can be loaded into the new train and continue on their journey to the original destination.
        let salvaged_cargo_ids = surviving_cars.iter().filter_map(|car| car.cargo.as_ref().map(|cargo| cargo.id)).collect::<Vec<u32>>();
        

        //processes cars and returns any issues to report, such as cars that failed intake and had to be moved to purgatory. We can include this information in the MissionReport to provide transparency about the salvage operation and any losses incurred.
        let stranded_issues = self.process_cars(surviving_cars, Some(mission_id)); // We can extract this logic into a separate method to keep things cleaner, and it can return the ledger of any failed cars for reporting.

        let reason: String = if stranded_issues.is_empty() {
            //"Engine lost, but all surviving cars were successfully salvaged.".to_string()
            format!("Engine lost, but all surviving cars were successfully salvaged. Salvaged cargo IDs: {:?}.", salvaged_cargo_ids)
        } else {
            format!("Engine lost, and some cars failed intake and sit in purgatory: {:?}.", stranded_issues)
        };
        let replacement_freight_order: FreightOrder = FreightOrder {
            id: mission_id, // Use the existing mission ID for the replacement freight order
            cargo_ids: salvaged_cargo_ids,
            destination,
            origin: self.id,
            ttl: 5,
        };

        let mut ledger_access = self.ledger.lock().unwrap();
        ledger_access.pending_cargo.push(replacement_freight_order);

        self.send_partial_failure_report(mission_id, &reason, &stranded_issues, report_to);
        self.print_status();
    }


    // This is the "loading phase" for incoming cars that are not part of a train. 
    fn handle_intake_cars(&mut self, cars: Vec<TrainCar>, reply_to: Option<Sender<Result<(), TrainError>>>) {
        println!("{BOLD}{CYAN}[{}] Populating yard with {} incoming cars from a perfectly standard, non-emergency source. It's not an emergency, promise!{RESET}", self.name, cars.len());
        let mut intake_issues = Vec::new();


        for car in cars {
            let car_id = car.id;
            match self.yard.receive_car(car) {
                Ok(Some(cargo)) => { 
                    
                    let mut destination: u32;
                    loop {
                        destination = rand::thread_rng().gen_range(0..=6);
                        if destination != self.id {
                            break;
                        }
                    }

                    let item_id = cargo.id;
                    self.warehouse.store(cargo); 

                    let freight_order = FreightOrder {
                        id: GLOBAL_ORDER_ID.fetch_add(1, Ordering::SeqCst), // Generate a new unique order ID for this cargo
                        cargo_ids: vec![item_id], // Create a freight order for this individual cargo item
                        destination, // Use the previously determined destination
                        origin: self.id,
                        ttl: 5,
                    };

                    let mut ledger_access = self.ledger.lock().unwrap();
                    ledger_access.pending_cargo.push(freight_order);
                },
                Ok(None) => {}, // Car is empty but safely in the yard
                Err((homeless_car, e)) => {
                    intake_issues.push(homeless_car.id);
                    println!("{RED} Failed to process Car {} during intake: {:?}. Moving to purgatory.{RESET}", car_id, e);
                    let rejected_asset = RejectedAsset::new(homeless_car, e, None); // We don't have a mission ID in this context, so we can pass None
                    self.yard.purgatory.push(rejected_asset);
                }
            }
        }

        if let Some(channel) = reply_to {
            if intake_issues.is_empty() {
                let _ = channel.send(Ok(()));
            } else {
                let _ = channel.send(Err(TrainError::ContrabandOnBoard(
                    format!("Cars to purgatory: {:?}", intake_issues),
                ))); // We can create a more specific error type for intake issues if we want, but for now we'll just use ContrabandOnBoard with an empty string, since the details of the error are already logged in the console.
            }
        }
    }

    fn handle_intake_cargo (&mut self, cargo: Vec<Cargo>, reply_to: Option<Sender<Result<(), TrainError>>>) {
        println!("{BOLD}{CYAN}[{}] Receiving {} cargo shipments into the warehouse.{RESET}", self.name, cargo.len());
        //create MutexGuard to access the ledger and log the incoming cargo.
        let mut ledger_access = self.ledger.lock().unwrap();
        for item in cargo {

            
            let mut destination: u32;
            loop {
                destination = rand::thread_rng().gen_range(0..=6);
                if destination != self.id {
                    break;
                }
            }

            let item_id = item.id;
            self.warehouse.store(item);

            ledger_access.pending_cargo.push(FreightOrder {
                id: GLOBAL_ORDER_ID.fetch_add(1, Ordering::SeqCst), // Generate a new unique order ID for this cargo
                cargo_ids: vec![item_id], // Create a freight order for this individual cargo item
                //set destination to a random station, u32 between 0 and 6
                destination, // Use the previously determined destination
                origin: self.id,
                ttl: 5,
            });

        }
        if let Some(channel) = reply_to {
            let _ = channel.send(Ok(()));
        }
    }

    pub fn handle_intake_engine(&mut self, engine: Engine, reply_to: Option<Sender<Result<(), TrainError>>>) {
        println!("{BOLD}{CYAN}[{}] Intaking engine {} of type {:?} into the roundhouse.{RESET}", self.name, engine.id, engine.engine_type);
        self.roundhouse.house(engine);
        if let Some(channel) = reply_to {
            let _ = channel.send(Ok(()));
        }
    }


    pub fn handle_new_neighbor(&mut self, neighbor: u32, tx: Sender<StationCommand>) {
        println!("{BOLD}{CYAN}[{}] Track connected to neighbor: {}.{RESET}", self.name, neighbor);
        self.neighbors.insert(neighbor, tx);
    }

    pub fn handle_request_empty_cars(&mut self, count: u32) {
        println!("{BOLD}{YELLOW}[{}] ⚠️ EMERGENCY LOGISTICS: Generating {} new empty cars from the ether...{RESET}", self.name, count);
        
        for _ in 0..count {
            // Grab a globally unique ID safely!
            let safe_id = self.yard.generate_new_car_id();
            
            let new_car = TrainCar {
                id: safe_id, 
                cargo: None,
                passenger: None,
            };
            
            //let _ = self.yard.receive_car(new_car); 
            if let Err((homeless_car, error)) = self.yard.receive_car(new_car) {
                println!("{RED}Failed to receive generated empty car with ID {}: {:?}. Moving to purgatory.{RESET}", safe_id, error);
                let rejected_asset = RejectedAsset::new(homeless_car, error, None);
                self.yard.purgatory.push(rejected_asset);
            }
        }
    }

    pub fn handle_engine_request(&mut self, requester_id: u32, request_id: u32, mission_id: Option<u32>, min_capacity: f64, mut mission_max_hop: f64, mut ttl: u32, mut branch_notified: [u32; 64], mut notified_count: usize) {
        println!("{BOLD}{YELLOW}[{}]::Station {}: Received engine request {} for mission ID {:?} for an engine with minimum capacity {}kg, mission max hop {}km, and TTL {} from Station {}.{RESET}", self.name, self.id, request_id, mission_id, min_capacity, mission_max_hop, ttl, requester_id);
        // check the number of engines of ANY TYPE across the entire roundhouse. We cannot give away our last engine, so we need to make sure we have at least 2 engines before we can fulfill this request. If we have 2 or more engines, we can send one to the requester. If we only have 1 engine, we cannot fulfill the request without risking our own operations, so we will have to decline.
        // we will iterate across the hashmap of engine types and count the total number of engines available. If the total number is greater than 1, we can fulfill the request. If the total number is 1 or less, we cannot fulfill the request.
        
        ttl -= 1; // Decrement TTL at the start of the method to ensure that we account for the hop to this station, even if we end up not forwarding the request due to lack of engines or TTL expiration. This way, the TTL accurately reflects the number of hops the request has taken through the network, regardless of whether it gets forwarded or not.

        if self.seen_engine_request.contains(&request_id) {
            println!("{YELLOW}Already processed engine request {}. Ignoring to prevent loops.{RESET}", request_id);
                return;
            } else {
                self.seen_engine_request.insert(request_id);
            }
            
        let total_engines_available: usize = self.roundhouse.stalls.iter().map(|(_, engines)| engines.len()).sum();
        let route_to_requester = match self.map.find_shortest_path(self.id, requester_id) {
            Some((_, r)) => r,
            None => {
                println!("{RED}Network Error: No track laid between {} and {}. Cannot fulfill engine request.{RESET}", self.id, requester_id);
                return;
            }
        };
        //let max_hop_to_requester = route_to_requester.windows(2).filter_map(|pair| self.map.get_distance(pair[0], pair[1])).fold(0./0., f64::max); // Calculate the max hop distance to the requester, which is needed to determine if we have a suitable engine that can make it there.
        for i in 0..route_to_requester.len() - 1 {
            if let Some(dist) = self.map.get_distance(route_to_requester[i], route_to_requester[i+1]) {
                if dist > mission_max_hop {
                    mission_max_hop = dist;
                }
            }
        }
        
        if total_engines_available > 1 {
            match self.roundhouse.find_suitable_engine(min_capacity, mission_max_hop){
                Ok(engine) => {
                    println!("{GREEN}Roundhouse {}: Found suitable engine {} for requester {} for request {}. Dispatching...{RESET}", self.id, engine.id, requester_id, request_id);
                    // We can dispatch the engine to the requester using the network's routing logic, which will find the best path from this station to the requester and send the engine along that path. We can create a temporary Train with just the engine and no cars to represent this transfer.
                    let temp_train = Train {
                        id: self.yard.generate_new_train_id(),
                        engine,
                        cars: Vec::new(),
                        mission_id,
                        destination: requester_id,
                        report_to: None,
                    };
                    self.dispatch_train(temp_train, route_to_requester);

                    // After dispatching the engine, we need to check if we should forward the request to our neighbors to see if they can also fulfill it, in case the requester needs multiple engines or if the requester is actually looking for an engine that meets the minimum capacity but also has other specific requirements that this engine doesn't meet. We can use the TTL to determine if we should forward the request, and we can use the branch_notified array to keep track of which neighbors have already been notified about this request to prevent loops. We will only forward the request if the TTL is greater than 0, and we will decrement the TTL before forwarding. We will also add this station's ID to the branch_notified array before forwarding, and we will increment the notified_count to keep track of how many neighbors have been notified.
                    println!("{YELLOW}Roundhouse {}: Checking if we should forward the engine request to neighbors after dispatching an engine to requester {}.{RESET}", self.id, requester_id);
                    if ttl > 0 {
                        //ttl -= 1; // Decrement TTL before forwarding
                        self.forward_engine_request(
                            requester_id,
                            request_id,
                            mission_id,
                            min_capacity,
                            mission_max_hop,
                            ttl,
                            branch_notified,
                            notified_count,
                        );
                    } else {
                        println!("{RED}[Station {} Roundhouse]: TTL expired for engine request from Station {}.{RESET}", self.id, requester_id);
                    }
                


                },
                Err(e) => {
                    println!("{RED}Roundhouse {} Error: Failed to find suitable engine for request from Station {}: {:?}.{RESET}", self.id, requester_id, e);
                    if ttl > 0 {
                        //ttl -= 1; // Decrement TTL before forwarding
                        self.forward_engine_request(
                            requester_id,
                            request_id,
                            mission_id,
                            min_capacity,
                            mission_max_hop,
                            ttl,
                            branch_notified,
                            notified_count,
                        );
                    } else {
                        println!("{RED}[Station {} Roundhouse]: TTL expired for engine request from Station {}.{RESET}", self.id, requester_id);
                    }
                
                }
            }
        } else {


            println!("{RED}Roundhouse {}: Only {} engine(s) available. Cannot fulfill request from Station {} without risking own operations.{RESET}", self.id, total_engines_available, requester_id);

            if ttl > 0 {
                //ttl -= 1; // Decrement TTL before forwarding
                self.forward_engine_request(
                    requester_id,
                    request_id,
                    mission_id,
                    min_capacity,
                    mission_max_hop,
                    ttl,
                    branch_notified,
                    notified_count,
                );
            } else {
                println!("{RED}[Station {} Roundhouse]: TTL expired for engine request from Station {}.{RESET}", self.id, requester_id);
            }
        
        }

    }


    fn initiate_engine_request(&mut self, requester_id: u32, request_id: u32, mission_id: Option<u32>, min_capacity: f64, mission_max_hop: f64, ttl: u32) {
        // We initialize branch_notified with the ID of the requester to prevent the request from being forwarded back to the requester and creating loops right from the start. We also initialize notified_count to 1 since we have already "notified" the requester by receiving the request in the first place.
        println!("{YELLOW}Roundhouse {}: Initiating engine request for Station {} with request ID {} for mission ID {:?}.{RESET}", self.id, requester_id, request_id, mission_id);
        let branch_notified = [requester_id; 64]; // We can use this array to keep track of which stations have been or will be notified of this request. Before forwarding this request, the station will place its id, as well the target stations' ids, into the array to prevent those stations from forwarding the request back to this station and creating loops. We initialize it with the requester_id to prevent loops right from the start.
        let notified_count = 1; // We start with 1 because we have already "notified" the requester by receiving the request in the first place.
        
        self.forward_engine_request(requester_id, request_id, mission_id, min_capacity, mission_max_hop, ttl, branch_notified, notified_count);
    }

    // Copilot, let's make a helper method for forwarding engine_requests to neighbors. We'll need to do it for the origin of the request, and we will need it for multiple arms of handle_engine_request when we have to forward due to insufficient engines or when we have to fan out due to TTL. This method will take care of stamping the branch_notified array and forwarding the request to the appropriate neighbors based on the TTL and the number of valid candidates. As well as incrementing the notified_count and ensuring we don't forward to neighbors that have already been notified. You got it, Copilot!
    fn forward_engine_request(&self, requester_id: u32, request_id: u32, mission_id: Option<u32>, min_capacity: f64, mission_max_hop: f64, ttl: u32, branch_notified: [u32; 64], notified_count: usize) {
        use rand::seq::SliceRandom; // We can use this to randomly select neighbors to forward the request to if we have to fan out due to TTL and number of candidates.
        

        //1. Discovery. First, we need to discover which neighbors are valid candidates for forwarding this request. Valid candidates are neighbors that have not already been notified about this request, which we can check using the branch_notified array and the notified_count to determine how many neighbors have already been notified.
        let mut valid_candidates: Vec<u32> = Vec::new(); // We can use this vector to store the valid candidates for forwarding the request, which are neighbors that have not already been notified about this request (to prevent loops). 

        let slice = &branch_notified[..notified_count]; // We can use this slice to check which neighbors have already been notified about this request. We only need to check the portion of the array that has been filled with notified neighbors, which is determined by the notified_count.

        for (id, _) in &self.neighbors {
            if !slice.contains(id) {
                valid_candidates.push(*id);
                
            }
        }

        // 2. Determine Fan-Out. (The MIN) We need to determine how many neighbors to forward the request to based on the TTL and the number of valid candidates. We can only forward to as many neighbors as the TTL allows, and we also need to make sure we don't try to forward to more neighbors than we have available.
        let fan_out = std::cmp::min(ttl as usize, valid_candidates.len()); // We can only forward to as many neighbors as the TTL allows, and we also need to make sure we don't try to forward to more neighbors than we have available, so we take the minimum of TTL and the number of valid candidates.

        if fan_out == 0 {
            println!("{RED}[Station {} Roundhouse]: No valid neighbors to forward engine request for Station {}. Cannot fulfill request without risking own operations.{RESET}", self.id, requester_id);
            return;
        }
        //3. Selection. We can randomly select neighbors from the valid candidates to forward the request to, up to the number allowed by the fan_out calculation. This random selection helps distribute the requests more evenly across the network and prevents certain stations from being overwhelmed with requests.
        let mut rng = rand::thread_rng();
        valid_candidates.shuffle(&mut rng); // We can shuffle the valid candidates to randomize which neighbors we forward to, to help distribute the requests more evenly across the network and prevent certain stations from being overwhelmed with requests.
        let chosen_candidates = &valid_candidates[..fan_out]; // We take a slice of the valid candidates based on the fan_out number we calculated, which is determined by the TTL and the number of valid candidates.

        // 4. Stamp the payload! Before we forward this request to the chosen neighbors, we need to stamp branch_notified with the IDs of the neighbors we are forwarding to.
        let mut next_notified = branch_notified; // We can create a mutable copy of the branch_notified array to modify for the next hop.
        let mut next_notified_count = notified_count; // We also need to keep track of how many neighbors have been notified so far.
        for &recipient_id in chosen_candidates {
            if next_notified_count < next_notified.len() { // We need to check this to prevent out-of-bounds errors in case we have a large number of neighbors and the branch_notified array is not large enough to hold all of them.
                next_notified[next_notified_count] = recipient_id;
                next_notified_count += 1;
            }
        }

        // 5. Distribute. We can now forward the request to the chosen neighbors with the stamped payload and assigned TTL.

        // First, let's determine how to split TTL among each chosen neighbor.

        let base_ttl = ttl/ chosen_candidates.len() as u32; // First, we calculate the base TTL for each neighbor.
        let ttl_remainder = ttl % chosen_candidates.len() as u32; // We also calculate the remainder of the TTL division, which we will distribute to the first few neighbors of the shuffled valid candidates to simulate randomness in TTL assignment.

        // for chosen_id in chosen_candidates {
        //     println!("{YELLOW} [{}]: Forwarding engine request to neighbor {} with base TTL {} and remainder TTL {} for request from Station {}.{RESET}", self.name, chosen_id, base_ttl, ttl_remainder, requester_id);
        //     let assigned_ttl = if ttl_remainder > 0 {
        //         ttl_remainder -= 1;
        //         base_ttl + 1
        //     } else {
        //         base_ttl
        //     };

        //     match self.neighbors.get(&chosen_id) {
        //         Some(neighbor) => {
        //             match neighbor.send(StationCommand::EngineRequest { 
        //                 requester_id, 
        //                 request_id,
        //                 min_capacity, 
        //                 mission_max_hop, 
        //                 ttl: assigned_ttl,
        //                 branch_notified: next_notified, // We forward the stamped branch_notified array to prevent loops.
        //                 notified_count: next_notified_count, // We also forward the updated count of how many neighbors have been notified so far.
        //             }) {
        //                 Ok(_) => println!("{YELLOW}[{}] Forwarded engine request to neighbor {} due to insufficient engines for request from Station {}.{RESET}", self.name, chosen_id, requester_id),
        //                 Err(e) => println!("{RED}[{}] DEAD-LETTER: Failed to forward engine request to neighbor {} for Station {}. Error: {:?}{RESET}", self.name, chosen_id, requester_id, e),
        //             }
        //         },
        //         None => println!("{RED}Network Error: Neighbor {} not found in neighbors list of Station {}. Cannot forward engine request.{RESET}", chosen_id, self.name),
        //     }       
        // }


        for (i, &chosen_id) in chosen_candidates.iter().enumerate() {
            //First, everyone gets the same base TTL, and then we distribute the remainder TTL to the first few neighbors in the shuffled list to add some randomness to the TTL assignment.
            let assigned_ttl = if i < ttl_remainder as usize {
                base_ttl + 1
            } else {
                base_ttl
            };
            println!("{YELLOW} [{}]::Station{}: Forwarding engine request to neighbor {} with assigned TTL {} for request from Station {}.{RESET}", self.name, self.id, chosen_id, assigned_ttl, requester_id);
            match self.neighbors.get(&chosen_id) {
                Some(neighbor) => {
                    match neighbor.send(StationCommand::EngineRequest { 
                        requester_id, 
                        request_id,
                        mission_id, // We also need to forward the mission_id in case we need to correlate this engine request with a specific mission at the requester station.
                        min_capacity,
                        mission_max_hop, 
                        ttl: assigned_ttl,
                        branch_notified: next_notified, // We forward the stamped branch_notified array to prevent loops.
                        notified_count: next_notified_count, // We also forward the updated count of how many neighbors have been notified so far.
                    }) {
                        Ok(_) => println!("{YELLOW}[{}] Forwarded engine request {} for mission ID {:?} to neighbor {} for request from Station {}.{RESET}", self.name, request_id, mission_id, chosen_id, requester_id),
                        Err(e) => println!("{RED}[{}] DEAD-LETTER: Failed to forward engine request {} for mission ID {:?} to neighbor {} for Station {}. Error: {:?}{RESET}", self.name, request_id, mission_id, chosen_id, requester_id, e),
                    }
                },
                None => println!("{RED}Network Error: Neighbor {} not found in neighbors list of Station {}. Cannot forward engine request {} for mission ID {:?}.{RESET}", chosen_id, self.name, request_id, mission_id),
            }

        }


    }




    pub fn check_pending_missions(&mut self) {
        if !self.pending_missions.is_empty() {
            println!("{YELLOW} Heartbeat Check: Station {} has {} pending missions waiting for resources. Attempting to retry... {RESET}", self.name, self.pending_missions.len());
            self.retry_pending_missions();
        }
    }




    pub fn print_status(&self) {
        println!("{BOLD}{CYAN}--- Station Status: {} ---{RESET}", self.name);
        self.yard.print_report(&self.roundhouse);
        println!("{BOLD}{YELLOW}Warehouse Inventory ({}){RESET}", self.warehouse.inventory.len());
        for (id, cargo) in &self.warehouse.inventory {
            println!("  -id: {}, item: {} ({}kg)", id, cargo.item, cargo.actual_weight);
        }
    }
        



    fn send_mission_failure(&self, mission_id: u32, error: TrainError, reply_to: Sender<Result<Train, TrainError>>) {
        if reply_to.send(Err(error)).is_err() {
            println!("{RED}Network Error: Failed to send mission failure report for Mission {}.{RESET}", mission_id);
        }
    }


    // This is a helper method for processing incoming cars, both from train arrivals and from external sources. It attempts to receive each car into the yard, and if the car contains cargo, it moves the cargo into the warehouse. If any issues arise during this process (such as contraband detection or other intake errors), it logs the issue, moves the car to purgatory, and collects the IDs of any cars that failed intake to include in the MissionReport for transparency.
    pub fn process_cars (&mut self, cars: Vec<TrainCar>, mission_id: Option<u32>) -> Vec<u32> {
        let mut failed_ids = Vec::new(); // We can fill this with any issues that arise during processing, and then include it in the MissionReport for transparency and debugging. For now, we'll just keep it empty to represent a perfect process.
        for car in cars {
            let car_id_we_just_received = car.id; // Store the ID before we potentially move the car into purgatory
            match self.yard.receive_car(car) {
                Ok(Some(cargo)) => { self.warehouse.store(cargo); },
                Ok(None) => {}, // Car is empty but safely in the yard
                Err((homeless_car, e)) => {
                    println!("{RED}Failed to process Car {} during intake: {:?}. Moving to purgatory.{RESET}", car_id_we_just_received, e);
                    failed_ids.push(homeless_car.id); // Log the ID of the car that caused issues for transparency
                    let rejected_asset = RejectedAsset::new(homeless_car, e, mission_id);
                    self.yard.purgatory.push(rejected_asset);
                }
            }
        }

        failed_ids

    }

    // helper method for the "dispatch train" phase of the mission. This is where we spawn a thread to simulate the train's journey to the next station, and we handle the logic for potential derailments during transit.
    pub fn dispatch_train(&self, mut train: Train, route: Vec<u32>) {
        let final_destination = train.destination;
        let station_tx_clone = self.tx.clone(); // Clone the station's own Sender for use in this method, so we can send SOS if needed

        let next_stop = route.get(1).cloned().unwrap_or_else(|| final_destination); // The next stop is the second element in the route (index 1), or the final destination if the route is just one stop
        let next_stop_handle = self.neighbors.get(&next_stop).expect("Next stop must be a neighbor").clone(); // Get the Sender for the next stop
        let distance_to_next_stop = self.map.get_distance(self.id, next_stop).expect("Distance to next stop must be calculable");

        let train_id = train.id; // Store the train ID for logging inside the thread
        let station_name_clone = self.name.clone(); // Clone the station name for use in this thread
        let station_id_clone = self.id.clone();
        let (transit_tx, transit_rx) = mpsc::channel();

        thread::spawn(move || {
            let time = train.dispatch(distance_to_next_stop).expect("Failed to dispatch");
            println!("{BOLD}{YELLOW}[{}::Station {}: Train {} is en route on Mission {} to next stop [Station {}]. Estimated time: {:.2} seconds.{RESET}", station_name_clone, station_id_clone, train_id, train.mission_id.unwrap_or(0), next_stop, time);
            thread::sleep(std::time::Duration::from_secs_f64(time)); // Simulate travel time to the next station. In a real implementation, this would be based on distance and train speed.

            // Using rand, simulate the train crashing with a 10% chance during transit. If it crashes, we issue a Derailment report back to transit_rx and skip the rest of the transit logic. The train is lost, so we don't send it to the next station. However, we return the salvaged TrainCars back to the yard for processing, and we send a MissionReport::Failure back to the mission's reply channel with details of the crash.
            let tree_falls = rand::thread_rng().gen_bool(0.1);
            if tree_falls {
                println!("{RED}🚨 DERAILMENT: Train {}!{RESET}", train_id);

                // We send an SOS command BACK to the Station's main mailbox!
                // (You will need to pass a clone of the Station's own Sender into the thread)
                station_tx_clone.send(StationCommand::HandleEmergencySOS {
                    mission_id: train.mission_id.unwrap_or(0),
                    destination: train.destination,
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
                    println!("{BOLD}{CYAN}[{}]::Station {}: CHOO CHOO! Train {} has been received at {}. Finalizing transit...{RESET}", station_name_clone, station_id_clone, train_id, next_stop);
                    // Here we would handle the result of the transit, such as sending a MissionReport back to the mission's reply channel based on success or failure at the next station.
                },
                Err(e) => {
                    println!("{RED}[{}] ERROR receiving transit confirmation for Train {}: {:?}{RESET}", station_name_clone, train_id, e);
                }

            }
        });                  
    }
    
    //helper method for sending failure reports to the mission's reply channel, to avoid repeating this logic in multiple places.
    pub fn report_mission_failure(&self, mission: &Mission, error_details: &str) {
        match &mission.reply_channel {
            Some(sender) => {
                let report = MissionReport::Failure(format!(
                    "Mission {} failed at {}: {}",
                    mission.id, self.name, error_details
                ));
                let _ = sender.send(report);
            },
            None => {
                println!("{RED}[{}] DEAD-LETTER: No reply channel to report failure for mission {} ({}){RESET}", 
                    self.name, mission.id, error_details);
            }
        }
    }

}


