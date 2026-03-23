use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::mpsc::Sender;

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";




#[derive(Debug)]
pub struct Cargo{
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
    pub fn max_capacity(&self) -> u32 {
        match self {
            EngineType::Percy => 5000,
            EngineType::Thomas => 15000,
            EngineType::Gordon => 50000,
            EngineType::Diesel => 20000,
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
    pub fn calculate_fuel_requirement(&self, weight: u32, distance: u32) -> f32 {
        let work = weight as f32 * distance as f32;
        let quotient = self.engine_type.fuel_efficiency() * 5000.0;
        work / quotient
    }

    pub fn can_complete_mission(&self, weight: u32, distance: u32) -> bool {
        let needed = self.calculate_fuel_requirement(weight, distance);
        
        if needed > self.current_fuel {
            println!("{RED}Mission Impossible: Engine {} needs {:.1}, has {:.1}{RESET}", self.id, needed, self.current_fuel);
            false
        } else {
            println!("{GREEN}Mission Possible: Engine {} ready!{RESET}", self.id);
            true
        }
    }

    pub fn burn_fuel(&mut self, weight: u32, distance: u32) {
        let needed = self.calculate_fuel_requirement(weight, distance);
        self.current_fuel -= needed;
        println!("{YELLOW}Engine {} consumed {:.1} fuel. Tank: {:.1}{RESET}", self.id, needed, self.current_fuel);
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
    pub distance_km: u32, // We can add more fields here as needed, like destination, mission details, etc.
    pub mission_id: Option<u32>, // We can link this train to a specific mission if we want to track that way.
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
    pub fn dispatch(&mut self) -> Result<(), TrainError> {
        println!("Train {} is departing for ({}km)...", self.id, self.distance_km);
        
        // 1. Calculate the final weight
        let total_weight = self.calculate_cargo_weight();
        
        // 2. The Consequence
        self.engine.burn_fuel(total_weight, self.distance_km);
        

        Ok(())
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

}

#[derive(Debug)]
#[derive(Clone)]
pub struct Mission {
    pub id: u32,
    pub request_id: u64,
    pub origin: String,
    pub destination: String,
    pub required_cars: Vec<u32>,
    //Sending a channel with the mission report back to the main thread so it can print the station status after the mission is processed.
    pub reply_channel: Option<Sender<MissionReport>>,
}


#[derive(Debug)]
pub enum MissionReport {
    Success(String),
    Failure(String),
}


#[derive(Debug)]
pub enum StationCommand {
    AssembleMission {
        mission: Mission,
        distance: u32,
        reply_to: Sender<Result<Train, TrainError>>,
    },
    ReceiveTrain {
        train: Train,
        reply_to: Sender<Result<(), TrainError>>,
    },
    IntakeCar {
        train_car: TrainCar,
        reply_to: Sender<Result<(), TrainError>>,
    },
    IntakeEngine {
        engine: Engine,
        reply_to: Sender<Result<(), TrainError>>,
    },
    PrintStatus,                   // Reporting
    Terminate,                     // Graceful Shutdown
}
