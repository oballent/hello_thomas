use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::mpsc::Sender;
use tracing::{debug, info, warn};
//use crate::facilities::{StationTx, StationRx};

//use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::mpsc::{self as tokio_mpsc, UnboundedSender, UnboundedReceiver,};
use tokio::sync::oneshot::{self as tokio_oneshot, Sender as OneShotSender, Receiver as OneShotReceiver,};

pub type StationTx = UnboundedSender<StationCommand>;
pub type StationRx = UnboundedReceiver<StationCommand>;

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";

pub const STATION_HEARTBEAT_MS: u64 = 500;

// Priority quotient for scheduling:
// (current_time - created_time) / (expire_time - created_time)
// Returned in parts-per-million (0..=1_000_000), clamped to [0, 1].
pub fn priority_quotient_ppm(created_time_ms: u64, expiry_time_ms: u64, now_ms: u64) -> u64 {
    let lifespan_ms = expiry_time_ms.saturating_sub(created_time_ms);
    if lifespan_ms == 0 {
        return 1_000_000;
    }

    let elapsed_ms = now_ms
        .saturating_sub(created_time_ms)
        .min(lifespan_ms);

    ((elapsed_ms as u128 * 1_000_000u128) / lifespan_ms as u128) as u64
}

pub fn decay_progress_ratio_ppm(created_time_ms: u64, expiry_time_ms: u64, now_ms: u64) -> u64 {
    priority_quotient_ppm(created_time_ms, expiry_time_ms, now_ms)
}















#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cargo{
    pub id: u32,
    pub item: String,
    pub destination: u32,
    pub actual_weight: u32,
    pub contraband: Option<String>,
    pub created_time_ms: u64,
    pub expiry_time_ms: u64,
    pub in_transit_since_ms: Option<u64>, // Decay is paused while cargo is in transit.
}


impl Cargo {
    pub fn dispatch_priority_ppm(&self, now_ms: u64) -> u64 {
        decay_progress_ratio_ppm(self.created_time_ms, self.expiry_time_ms, now_ms)
    }

    pub fn is_expired(&self, now_ms: u64) -> bool {
        now_ms >= self.expiry_time_ms
    }

    // We use &mut self because we are going to "reach in and grab" the item
    pub fn check_and_confiscate(&mut self) -> Result<String, TrainError> {
        
        // .take() effectively "steals" the contraband out of the cargo
        // and leaves a None in its place.
        if let Some(seized_item) = self.contraband.take() {
            warn!("SECURITY: Confiscated '{}' from cargo!", seized_item);
            
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
    pub timestamp: u64, // When did it fail?
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
            EngineType::Percy => 6000.0,
            EngineType::Thomas => 16500.0,
            EngineType::Gordon => 53000.0,
            EngineType::Diesel => 22000.0,
        }
    }

    pub fn weight(&self) -> f64 {
        match self {
            EngineType::Percy => 1000.0,
            EngineType::Thomas => 1500.0,
            EngineType::Gordon => 3000.0,
            EngineType::Diesel => 2000.0,
        }
    }

    pub fn ideal_min_capacity(&self) -> f64 {
        match self {
            EngineType::Percy => 0.0,
            EngineType::Thomas => 2500.0,
            EngineType::Gordon => 19500.0,
            EngineType::Diesel => 8000.0,
        }
    }
    
    pub fn max_fuel_capacity(&self) -> f32 {
        // Let's assume these units are 'Liters' or 'Kilograms of Coal'
        match self {
            EngineType::Percy => 1500.0,
            EngineType::Thomas => 2500.0,
            EngineType::Diesel => 3000.0,
            EngineType::Gordon => 5000.0,
        }
    }

    pub fn fuel_efficiency(&self) -> f32 {
        // Higher is better. 
        // A Diesel might get 5.0 km/kg of fuel per ton.
        // A Thomas (Steam) might only get 2.5 km/kg.
        match self {
            EngineType::Diesel => 0.60, // Devious, but extremely efficient
            EngineType::Percy => 0.35, //  Smart and efficient, but not the strongest
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

    pub fn is_ideal_for_mission(&self, freight_weight: u32) -> bool {
        let projected_train_weight = freight_weight as f64 + self.engine_type.weight(); // We want to consider the weight of the engine itself in our fuel calculations.
        let capacity = self.engine_type.max_capacity();
        let ideal_min = self.engine_type.ideal_min_capacity();
        projected_train_weight > ideal_min && projected_train_weight <= capacity
    }

    pub fn can_complete_mission(&self, freight_weight: u32, distance: f64) -> bool {

        let projected_train_weight = freight_weight as f64 + self.engine_type.weight(); // We want to consider the weight of the engine itself in our fuel calculations.

        let needed = self.calculate_fuel_requirement(projected_train_weight, distance);

        let feasible = needed <= self.current_fuel;
        debug!(
            "Feasibility check: engine={}, freight_weight={}, distance={:.2}, needed={:.1}, fuel={:.1}, feasible={}",
            self.id,
            freight_weight,
            distance,
            needed,
            self.current_fuel,
            feasible
        );
        feasible
    }

    pub fn burn_fuel(&mut self, freight_weight: f64, distance: f64) -> Result<(), TrainError> {
        let projected_train_weight = freight_weight + self.engine_type.weight(); // Consider the weight of the engine itself in our fuel calculations.
        let needed = self.calculate_fuel_requirement(projected_train_weight, distance);
        if needed > self.current_fuel {
            Err(TrainError::MissionImpossible {
                reason: format!("Engine {} needs {:.1} fuel, has {:.1} fuel", self.id, needed, self.current_fuel),
            })
        } else {
            self.current_fuel -= needed;
            info!("Engine {} consumed {:.1} fuel at projected train weight {:.1}. Tank: {:.1}", self.id, needed, projected_train_weight, self.current_fuel);
            Ok(())
        }
    }

    pub fn refuel(&mut self) {
        let max = self.engine_type.max_fuel_capacity();
        if self.current_fuel < max {
            self.current_fuel = max;
            info!("⛽ Engine {} refueled to max capacity ({:.1}).", self.id, max);
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
            info!("UNLOADING: Car {} is discharging its payload {}.", self.id, cargo.item);
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
    pub mission_id: Option<u32>, // This is optional because a train might be in the process of being assembled and not have a mission yet, or it might be between missions.
    pub request_id: Option<u32>, // This is the ID of the engine request that led to this train being assembled. This allows us to correlate the train back to the original request and mission, which is useful for reporting and debugging.
    pub responder_id: Option<u32>, // This is the ID of the station that provided the engine for this requested emergency engine. If the train derails, we can include this in the failed engine transfer report to the requesting station.
    // Now, for actor-based, decentralized travel across shortest route to destination
    //pub route_to_destination: Vec<String>, // A list of station names representing the planned route. This is based off the network's pathfinding algorithm. We will use this to know where to send the train next, and to report back to the mission with the path taken.
    pub destination: u32, // The final destination station name. This is used for reporting back to the mission and for the train's internal logic to know when it has arrived.
    pub report_to: Option<OneShotSender<MissionReport>>,
    pub engine_request_reply_to: Option<StationTx>,
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
        info!("Train {}::Engine {} is departing for ({}km)...", self.id, self.engine.id, distance_to_next_stop);
        
        // 1. Calculate the final weight
        let freight_weight = self.calculate_gross_weight(); // Convert to u32 for fuel calculation. In a real system, we would want to be careful about potential overflows here and might want to use a larger integer type or a different approach to weight management.
        let speed = self.engine.engine_type.speed() as f64;
        
        // 2. The Consequence
        self.engine.burn_fuel(freight_weight, distance_to_next_stop)?;
        

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
pub enum MissionReport {
    Success(String),
    PartialFailure(String),
    Failure(String),
}


#[derive(Debug)]
pub enum StationCommand {
    Beat, // The Heartbeat: a regular tick that triggers the station to check its pending missions and take action. This is important for keeping the station's internal logic moving forward, such as checking for mission timeouts, re-evaluating pending missions, and generally keeping the station "alive" and responsive.
    ReceiveTrain {
        train: Train,
        //reply_to: Sender<Result<(), TrainError>>,
        reply_to: OneShotSender<Result<(), TrainError>>,
    },
    HandleEmergencySOS { 
        mission_id: u32, 
        destination: u32,
        surviving_cars: Vec<TrainCar>, 
        report_to: Option<OneShotSender<MissionReport>> 
    },
    IntakeCar {
        cars: Vec<TrainCar>,
        reply_to: OneShotSender<Result<(), TrainError>>,
    },
    IntakeCargo {
        cargo: Vec<Cargo>,
        reply_to: OneShotSender<Result<(), TrainError>>,
    },
    IntakeEngine {
        engine: Engine,
        reply_to: Option<OneShotSender<Result<(), TrainError>>>, // This is optional because sometimes we might want to just dump an engine into the station without waiting for a response
    },
    NewNeighbor {
        neighbor: u32,
        neighbor_tx: StationTx, //Sender<StationCommand>,
    },
    EngineRequest { 
        requester_id: u32,
        forwarder_id: u32, // The station that forwarded this request to us. This allows us to know where the request came from, and to all each station to trace its distance and path backwards.
        request_id: u32, // unique ID for this specific request
        mission_id: Option<u32>, // The mission this engine request is for, if applicable. This allows us to track which mission the request belongs to and include that information in our reporting and decision-making. It's optional because we might have some engine requests that are not tied to a specific mission, such as a station requesting an engine for general use or for a future mission that has not been fully defined yet.
        min_capacity: u32, // The minimum cargo weight that the engine needs to be able to handle. This allows the station to filter out engines that are too weak for the mission right from the start, which saves time and resources by not sending requests to stations that can't possibly fulfill them.
        max_hop_to_requester: f64, // The longest hop along the route to the requester, which is the distance the engine would have to travel empty to get to the cargo.
        mission_max_hop: f64, // NEW: The widest gap the engine will face BEFORE or AFTER it arrives to the requesting station. This allows the engine to consider not just whether it can get TO the requesting station, but if it can complete the requesting station's entire mission, which is the real question.
        ttl: u32,

        // THE FIX: A fixed-size array and a counter.
        // This lives entirely on the stack. Zero heap allocation!
        branch_notified: [u32; 64], // A list of ancestor stations and their neighbors that have already been notified about this request. This prevents us from wasting TTL on sending the same request to the same station multiple times.
        notified_count: usize,
        reply_to: StationTx,
    },
    // EngineRequestConfirmed {
    //     request_id: u32,
    //     mission_id: Option<u32>,
    //     responder_id: u32,
    //     engine_id: u32,
    // },
    EngineTransferFailed {
        request_id: u32,
        mission_id: Option<u32>,
        responder_id: u32,
        reason: String,
    },

    OfferEngine {
        request_id: u32,
        mission_id: Option<u32>,
        responder_id: u32,
        engine_id: u32,
        reply_to: OneShotSender<bool>, // The handshake channel!
    },
    CommitDispatch {
        train: Train,
        route: Vec<u32>,
    },
    
    CheckStatus, // The Alarm Clock: station sends to itself every X seconds to trigger a check of the pending missions list, which is stored locally at each station. 

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