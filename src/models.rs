use std::hash::Hash;
//use std::os::windows::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::mpsc::{self, Receiver, Sender};
use std::collections::HashMap;
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use crate::network::GlobalLedger;

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";








//#[derive(Clone)]
#[derive(Debug)]
// The "Ticket" on the Marketplace board
pub struct FreightOrder {
    pub id: u32,
    pub cargo_ids: Vec<u32>, // In a more complex system, a single freight order might involve multiple pieces of cargo that need to be shipped together. For simplicity, we can assume each freight order corresponds to one piece of cargo, but using a Vec allows for future expansion without changing the data structure.
    pub origin: u32, // So the Producer knows which tx channel to use!
    pub destination: u32,
    //pub description: String,
    //pub weight: u32,
    pub ttl: u32, // Time to live, in iterations of the Producer's while active loop.
}

pub struct Producer {
    pub id: u32,
    pub ledger: Arc<Mutex<GlobalLedger>>, // The source of truth for pending cargo and active missions. 
    pub switchboard: HashMap<u32, Sender<StationCommand>>, // Maps station IDs to their command channels
}

impl Producer {
    pub fn new(id: u32, ledger: Arc<Mutex<GlobalLedger>>, switchboard: HashMap<u32, Sender<StationCommand>>) -> Self {
        Producer {
            id,
            ledger,
            switchboard,
        }
    }

    pub fn start(self) -> JoinHandle<()> {
        thread::spawn(move || {
            println!("{CYAN}Producer {} is starting up...{RESET}", self.id);
            // We still pull the pending cargo from the ledger, but we do it inside the thread so that we have access to the switchboard and can send commands to the stations.
            
            let mut active_monitors: Vec<(Receiver<MissionReport>, FreightOrder)> = Vec::new(); // This pairs the mission report channels with their corresponding freight orders so we can keep track of which reports belong to which missions. Alternatively, we could just use missions, as mission is made up of freight order and the producer's reply channel. We'll probably change this later.
            let mut active: bool = true;

            while active {
                println!("while active loop start for Producer {}", self.id);
                // 2. Create a temporary variable to hold our assignment (if we get one)
                let my_assignment: Option<FreightOrder> = {
                    println!("Producer {} is waiting for the Talking Stick to check the Global Ledger for pending cargo...", self.id);
                    
                    // 3. Wait in line for the Talking Stick
                    // --- LOCK ACQUIRED ---
                    let mut ledger_access = self.ledger.lock().unwrap();
                    
                    // 4. We now have exclusive, mutable access to the GlobalLedger!

                    println!("There are currently {} items waiting to be shipped.", ledger_access.pending_cargo.len());
                    // We act like a Hungry Hippo: just pop the last item off the list.
                    // If the list is empty, pop() returns None.
                    ledger_access.pending_cargo.pop() 
                    
                }; // --- LOCK DROPPED AUTOMATICALLY HERE! ---
                // 5. Now we are outside the lock. The ledger is free for other threads.

                //if we got an assignment, we send it!
                if let Some(freight_order) = my_assignment {
                    println!("Producer {} claimed cargo IDs {:?}. Building mission...", self.id, freight_order.cargo_ids);

                    let (tx_report, rx_report) = mpsc::channel();


                    // Build our Mission for this single piece of cargo
                    
                    let mission = Mission {
                        id: freight_order.id, // We can use the freight order ID as the mission ID for simplicity, since each mission corresponds to a single freight order in this case. In a more complex system, we might want to have a separate ID generator for missions, but for this example, using the freight order ID works fine and keeps things straightforward.
                        request_id: (10 * freight_order.id) + freight_order.id, // Just an example of how we might generate a request ID based on the freight order ID. This is arbitrary and can be adjusted as needed.
                        origin: freight_order.origin,
                        destination: freight_order.destination,
                        cargo_ids: freight_order.cargo_ids.clone(), // Assuming each cargo requires one car with the same ID as the cargo for simplicity. In a real system, we would need more complex logic to determine which cars are needed for which cargo.
                        reply_channel: Some(tx_report.clone()), // The producer's channel to receive updates about this mission
                    };
                    
                    
                    if let Some(origin_tx) = self.switchboard.get(&freight_order.origin) {
                        println!("{CYAN}Producer {} is sending mission {} for cargo IDs {:?} to Station {}...{RESET}", self.id, mission.id, freight_order.cargo_ids, freight_order.origin);
                        
                        origin_tx.clone().send(StationCommand::AssembleMission { 
                            mission, // <-- Idiomatic Rust shorthand!
                        }).expect("Failed to send AssembleMission command over open channel");

                        // // The Tiny Intern Thread!
                        // thread::spawn(move || {
                        //     if let Ok(report) = rx_report.recv() {
                        //         println!("Producer {} received report: {:?}", self.id, report);
                        //     }
                        // });

                        active_monitors.push((rx_report, freight_order));// We can store our rx_report and wait for a response from tx_report outside the loop, which allows us to continue claiming missions and sending them to the stations without blocking on waiting for the reports. This is called batching, and it's a common technique in asynchronous programming to allow for more efficient use of resources and better responsiveness.

                    } else {
                        println!("{RED}Error: Radio channel for Station {} not found in switchboard! Reinserting freight order {:?} {RESET}", freight_order.origin, freight_order);// RE-INSERT INTO LEDGER!
                        let mut ledger_access = self.ledger.lock().unwrap();
                        // This will cause a bug; the origin's rx is missing from the switchboard, so this freight order will just keep getting reinserted and never processed. In a real system, we would want to have some error handling for this case, such as a retry mechanism or a way to alert the system administrators that there is a problem with the switchboard. For our simulation, we will just print an error message and reinsert the freight order back into the ledger, but we should be aware that this could lead to an infinite loop if the switchboard issue is not resolved. But for now, we'll be able to see the bug in action with our println!
                        ledger_access.pending_cargo.push(freight_order);
                    }

                }
                
                // 3. Now check all active missions to see if any reports came back
                let mut still_monitoring = Vec::new();
                for (rx, mut order) in active_monitors {
                    match rx.try_recv() {
                        Ok(MissionReport::Success(details)) => println!("{GREEN} Producer {} success: {}", self.id, details),
                        Ok(MissionReport::PartialSuccess(details)) => {
                            println!("{YELLOW} Producer {} partial success: {}", self.id, details);
                        }
                        Ok(MissionReport::Failure(details)) => { 
                            println!("{RED} Producer {} failure: {}", self.id, details);
                            // Optionally, we could reinsert the freight order back into the ledger here if we want to retry it later
                            let mut ledger_access = self.ledger.lock().unwrap();
                            order.ttl -= 1; // Decrement the TTL for this order since it failed. 
                            if order.ttl > 0 {
                                ledger_access.pending_cargo.push(order);
                            }
                        }
                        Err(mpsc::TryRecvError::Empty) => {
                            // No report yet, keep monitoring
                            still_monitoring.push((rx, order));
                        }
                        Err(mpsc::TryRecvError::Disconnected) => {
                            println!("{RED} Producer {} error: Station {} disconnected", self.id, order.origin);

                        }
                    }
                }
                active_monitors = still_monitoring; // Update our active monitors with the ones that are still pending

                // Check the Global Ledger again to see if there are more pending cargo items to claim. If not, we can set active to false
                let ledger_is_empty = {
                    let ledger_access = self.ledger.lock().unwrap();
                    ledger_access.pending_cargo.is_empty()
                };

                if ledger_is_empty && active_monitors.is_empty() {
                    println!("Producer {} has no more pending cargo to claim and no active missions to monitor. Clocking out.", self.id);
                    active = false;
                } else {
                    //sleep to avoid burning CPU cycles while waiting for Stations to report back. In a real system, we would want a more sophisticated event-driven approach rather than just sleeping, but this is fine for our simulation.
                    thread::sleep(std::time::Duration::from_millis(500));
                }


            }
        })
    }
}














#[derive(Debug)]
pub struct Cargo{
    pub id: u32,
    pub item: String,
    pub actual_weight: u32,
    pub contraband: Option<String>,
}


impl Cargo {
    // We use &mut self because we are going to "reach in and grab" the item
    pub fn check_and_confiscate(&mut self) -> Result<String, TrainError> {
        
        // .take() effectively "steals" the contraband out of the cargo
        // and leaves a None in its place.
        if let Some(seized_item) = self.contraband.take() {
            println!("{RED}SECURITY: Confiscated '{}' from cargo!{RESET}", seized_item);
            
            // We return an Error that OWNS the stolen string.
            // No references, no lifetimes, no dangling pointers.
            return Err(TrainError::ContrabandOnBoard(seized_item));
        }

        Ok(format!("Cargo '{}' is clear and safe.", self.item))
    }
}


pub struct RejectedAsset {
    pub car: TrainCar,
    pub issue: Vec<TrainError>,
    pub timestamp: u64, // When did it fail? How to impement this? A counter?
    pub source_mission: Option<u32>, // Where did it come from? Mission ID, or None?
}


impl RejectedAsset {
    pub fn new(car: TrainCar, issue: Vec<TrainError>, source_mission: Option<u32>) -> Self {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        Self {
            car,
            issue,
            timestamp,
            source_mission,
        }
    }
}



#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)] // This allows us to easily create copies of EngineType values, which is useful for passing them around without losing ownership.
pub enum EngineType {
    Diesel,
    Thomas,
    Percy,
    Gordon,
}


impl EngineType {
    pub fn max_capacity(&self) -> f64 {
        match self {
            EngineType::Percy => 5000.0,
            EngineType::Thomas => 15000.0,
            EngineType::Gordon => 50000.0,
            EngineType::Diesel => 20000.0,
        }
    }

    pub fn ideal_min_capacity(&self) -> f64 {
        match self {
            EngineType::Percy => 0000.0,
            EngineType::Thomas => 5000.0,
            EngineType::Gordon => 20000.0,
            EngineType::Diesel => 15000.0,
        }
    }
    
    pub fn max_fuel_capacity(&self) -> f32 {
        // Let's assume these units are 'Liters' or 'Kilograms of Coal'
        match self {
            EngineType::Percy => 1000.0,
            EngineType::Thomas => 2000.0,
            EngineType::Diesel => 3000.0,
            EngineType::Gordon => 5000.0,
        }
    }

    pub fn fuel_efficiency(&self) -> f32 {
        // Higher is better. 
        // A Diesel might get 5.0 km/kg of fuel per ton.
        // A Thomas (Steam) might only get 2.5 km/kg.
        match self {
            EngineType::Diesel => 0.50, // Devious, but extremely efficient
            EngineType::Percy => 0.30, //  Smart and efficient, but not the strongest
            EngineType::Thomas => 0.25, // Classic, Jack of all trades
            EngineType::Gordon => 0.18, // Powerful, but a gas guzzler
        }
    }

    pub fn speed(&self) -> u32 {
        match self {
            EngineType::Percy => 40*6,
            EngineType::Thomas => 60*6,
            EngineType::Gordon => 80*6,
            EngineType::Diesel => 70*6,
        }
    }
}


#[derive(Debug)]
pub enum TrainError {
    ContrabandOnBoard(String),
    DuplicateId(u32),
    // ... our existing variants ...
    NoAvailableEngine,
    AssemblyFailed {
        missing_car_ids: Vec<u32>,
        engine_returned: u32,
    },
    MissionImpossible {
        reason: String,
    },
    CarToPurgatory {
        car_id: u32,
        issues: String,
    },
    Derailment {
        mission_id: u32,
        surviving_cars: Vec<TrainCar>,
        last_known_station: String, 
        report_to: Option<Sender<MissionReport>>,
    },
    MissingCargo {
        cargo_id: Vec<u32>,
    }
}


#[derive(Debug)]
pub struct Engine {
    pub id: u32,
    pub engine_type: EngineType,
    pub current_fuel: f32, // Replaces fuel_level
    //pub max_fuel: f32,
}



impl Engine {
    /// THE SINGLE SOURCE OF TRUTH for fuel consumption math.
    pub fn calculate_fuel_requirement(&self, weight: f64, distance: f64) -> f32 {
        let work = weight as f32 * distance as f32;
        let quotient = self.engine_type.fuel_efficiency() * 5000.0;
        work / quotient
    }

    pub fn is_ideal_for_mission(&self, weight: f64) -> bool {
        let capacity = self.engine_type.max_capacity();
        let ideal_min = self.engine_type.ideal_min_capacity();
        weight > ideal_min && weight <= capacity
    }

    pub fn can_complete_mission(&self, weight: f64, distance: f64) -> bool {
        let needed = self.calculate_fuel_requirement(weight, distance);
        
        if needed > self.current_fuel {
            println!("{RED}Mission Impossible: Engine {} needs {:.1} fuel, has {:.1} fuel{RESET}", self.id, needed, self.current_fuel);
            false
        } else {
            println!("{GREEN}Mission Possible: Engine {} ready!{RESET}", self.id);
            true
        }
    }

    pub fn burn_fuel(&mut self, weight: f64, distance: f64) -> Result<(), TrainError> {
        let needed = self.calculate_fuel_requirement(weight, distance);
        if needed > self.current_fuel {
            Err(TrainError::MissionImpossible {
                reason: format!("Engine {} needs {:.1} fuel, has {:.1} fuel", self.id, needed, self.current_fuel),
            })
        } else {
            self.current_fuel -= needed;
            println!("{YELLOW}Engine {} consumed {:.1} fuel. Tank: {:.1}{RESET}", self.id, needed, self.current_fuel);
            Ok(())
        }
    }

    pub fn refuel(&mut self) {
        let max = self.engine_type.max_fuel_capacity();
        if self.current_fuel < max {
            self.current_fuel = max;
            println!("{GREEN}⛽ Engine {} refueled to max capacity ({:.1}).{RESET}", self.id, max);
        }
    }
}



#[derive(Debug)]
pub struct TrainCar {
    pub id: u32,
    pub cargo: Option<Cargo>,
    pub passenger: Option<String>,
}


impl TrainCar {
    pub fn calculate_cargo_weight(&self) -> u32 {
        self.cargo
            .as_ref()
            .map(|c| c.actual_weight)
            .unwrap_or(0)
    }

    pub fn gross_weight(&self) -> u32 {
        let tare_weight = 2000; // If cars have different weights later, make this a struct field.
        let net_weight = self.calculate_cargo_weight();
        
        tare_weight + net_weight
    }

    /// The 'Definition of Done'. Returns the cargo, leaving the car empty.
    pub fn unload_cargo(&mut self) -> Option<Cargo> {
        if let Some(cargo) = &self.cargo {
            println!("{CYAN}UNLOADING: Car {} is discharging its payload {}.{RESET}", self.id, cargo.item);
        }
        return self.cargo.take() // The magic of .take() again—ownership moves out!
    }
}



#[derive(Debug)]
pub struct Train{
    pub id: u32,
    pub cars: Vec<TrainCar>,
    pub engine: Engine, // Ownership! The Engine is PHYSICALLY in the Train now.
    //pub distance_km: f64, // We can add more fields here as needed, like destination, mission details, etc.
    pub mission_id: Option<u32>, // We can link this train to a specific mission if we want to track that way.
    // Now, for actor-based, decentralized travel across shortest route to destination
    //pub route_to_destination: Vec<String>, // A list of station names representing the planned route. This is based off the network's pathfinding algorithm. We will use this to know where to send the train next, and to report back to the mission with the path taken.
    pub destination: u32, // The final destination station name. This is used for reporting back to the mission and for the train's internal logic to know when it has arrived.
    pub report_to: Option<Sender<MissionReport>>
}

impl Train {
    
    pub fn eject_car(&mut self, id: u32) -> Option<TrainCar> {
        if let Some(pos) = self.cars.iter().position(|c| c.id == id) {
            Some(self.cars.remove(pos))
        } else {
            None
        }
    }
    

    // Notice the &mut self. The train is 'taking damage' (burning fuel).
    pub fn dispatch(&mut self, distance_to_next_stop: f64) -> Result<f64, TrainError> {
        println!("Train {}::Engine {} is departing for ({}km)...", self.id, self.engine.id, distance_to_next_stop);
        
        // 1. Calculate the final weight
        let total_weight = self.calculate_gross_weight(); // Convert to u32 for fuel calculation. In a real system, we would want to be careful about potential overflows here and might want to use a larger integer type or a different approach to weight management.
        let speed = self.engine.engine_type.speed() as f64;
        
        // 2. The Consequence
        self.engine.burn_fuel(total_weight, distance_to_next_stop)?;
        

        Ok(distance_to_next_stop / speed) // Return the estimated time to next stop based on speed
    }


    pub fn calculate_cargo_weight(&self) -> u32 {
        self.cars.iter()
            .map(|car|{
                match &car.cargo {
                    Some(cargo) => cargo.actual_weight,
                    None => 0,
                }
            })
            .sum()
    }

    pub fn calculate_gross_weight(&self) -> f64 {
        // Sum the gross weight of all attached cars
        let consist_weight: u32 = self.cars.iter().map(|car| car.gross_weight()).sum();
        
        // If you want the Engine's mass to burn fuel too, you add it here.
        // let engine_weight = 5000; 
        
        consist_weight as f64
    }

}

#[derive(Debug)]
#[derive(Clone)]
pub struct Mission {
    pub id: u32,
    pub request_id: u32,
    pub origin: u32,
    pub destination: u32,
    pub cargo_ids: Vec<u32>,
    //Sending a channel with the mission report back to the main thread so it can print the station status after the mission is processed.
    pub reply_channel: Option<Sender<MissionReport>>,
}


#[derive(Debug)]
pub enum MissionReport {
    Success(String),
    PartialSuccess(String),
    Failure(String),
}


#[derive(Debug)]
pub enum StationCommand {
    AssembleMission {
        mission: Mission,
    },
    ReceiveTrain {
        train: Train,
        reply_to: Sender<Result<(), TrainError>>,
    },
    HandleEmergencySOS { 
        mission_id: u32, 
        destination: u32,
        surviving_cars: Vec<TrainCar>, 
        report_to: Option<Sender<MissionReport>> 
    },
    IntakeCar {
        cars: Vec<TrainCar>,
        reply_to: Sender<Result<(), TrainError>>,
    },
    IntakeCargo {
        cargo: Vec<Cargo>,
        reply_to: Sender<Result<(), TrainError>>,
    },
    IntakeEngine {
        engine: Engine,
        reply_to: Sender<Result<(), TrainError>>,
    },
    NewNeighbor {
        neighbor: u32,
        neighbor_tx: Sender<StationCommand>,
    },
    RequestEmptyCars {
        count: u32,
    },
    EngineRequest { 
        requester_id: u32,
        request_id: u32, // unique ID for this specific request
        min_capacity: f64,
        mission_max_hop: f64, // NEW: The widest gap the engine will face BEFORE or AFTER it arrives to the requesting station. This allows the engine to consider not just whether it can get TO the requesting station, but if it can complete the requesting station's entire mission, which is the real question. An engine might be able to get to the station but then not have enough fuel to complete the next leg of the journey, so this gives us a more holistic view of whether the engine is truly suitable for the mission.
        ttl: u32,

        // THE FIX: A fixed-size array and a counter.
        // This lives entirely on the stack. Zero heap allocation!
        branch_notified: [u32; 64], // A list of ancestor stations and their neighbors that have already been notified about this request. This prevents us from wasting TTL on sending the same request to the same station multiple times.
        notified_count: usize,
    },
    EngineRequestResponse {
        request_id: u32, // This should match the request_id from the EngineRequest command so the requester can correlate responses to their original request.
        station_id: u32, // The ID of the station that is offering the engine. This allows the requester to know where the engine is coming from and potentially request it from that station if they want to.
        engine: Engine,
    },

    PrintStatus,                   // Reporting
    Terminate,                     // Graceful Shutdown
}


#[derive(Clone)]
pub struct Location {
    pub x: f64,
    pub y: f64,
}


impl Location {
    // A simple method to execute our math formula
    pub fn distance_to(&self, other: &Location) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}