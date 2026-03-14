



use std::collections::{HashMap, VecDeque};

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";

// This program demonstrates the concept of mutable references in Rust using a simple example of train engines and their personalities.


#[derive(Debug)]
struct Cargo{
    item: String,
    actual_weight: u32,
    contraband: Option<String>,
}

struct Engine {
    id: u32,
    engine_type: EngineType,
    current_fuel: f32, // Replaces fuel_level
    //max_fuel: f32,
}

#[derive(Debug)]
struct TrainCar {
    id: u32,
    cargo: Option<Cargo>,
    passenger: Option<String>,
}


struct RejectedAsset {
    car: TrainCar,
    issue: TrainError,
    timestamp: u64, // When did it fail? How to impement this? A counter?
    source_mission: Option<u32>, // Where did it come from? Mission ID, or None?
}

impl RejectedAsset {
    fn new(car: TrainCar, issue: TrainError, timestamp: u64, source_mission: Option<u32>) -> Self {
        Self { car, issue, timestamp, source_mission }
    }
}



struct Train{
    id: u32,
    cars: Vec<TrainCar>,
    engine: Engine, // Ownership! The Engine is PHYSICALLY in the Train now.
    distance_km: u32, // We can add more fields here as needed, like destination, mission details, etc.
    mission_id: Option<u32>, // We can link this train to a specific mission if we want to track that way.
}


struct Mission {
    id: u32,
    destination: String,
    required_cars: Vec<u32>,
    distance_km: u32,
}



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

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)] // This allows us to easily create copies of EngineType values, which is useful for passing them around without losing ownership.
enum EngineType {
    Diesel,
    Thomas,
    Percy,
    Gordon,
}

#[derive(Debug)]
enum TrainError {
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
}

// fn check_contraband(cargo: &Cargo) -> Result<String, TrainError> {
//     match &cargo.contraband {
//         Some(item) => Err(TrainError::ContrabandOnBoard(item.clone())), // We clone the string here to avoid taking ownership of it. This way, we can still use the original cargo object later if we need to.
//         None => Ok(String::from("No contraband detected in this cargo!")),
//     }
// }/testing


impl Cargo {
    // We use &mut self because we are going to "reach in and grab" the item
    fn check_and_confiscate(&mut self) -> Result<String, TrainError> {
        
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


impl TrainCar {
    fn calculate_cargo_weight(&self) -> u32 {
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
        self.cargo.take() // The magic of .take() again—ownership moves out!
    }
}


impl Engine {
    /// THE SINGLE SOURCE OF TRUTH for fuel consumption math.
    pub fn calculate_fuel_requirement(&self, weight: u32, distance: u32) -> f32 {
        let work = weight as f32 * distance as f32;
        let quotient = self.engine_type.fuel_efficiency() * 500.0;
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


impl Train {
    
    // Notice the &mut self. The train is 'taking damage' (burning fuel).
    fn dispatch(&mut self) -> Result<(), TrainError> {
        println!("Train {} is departing for ({}km)...", self.id, self.distance_km);
        
        // 1. Calculate the final weight
        let total_weight = self.calculate_cargo_weight();
        
        // 2. The Consequence
        self.engine.burn_fuel(total_weight, self.distance_km);
        
        // 3. (Future) We could clear the cars here, simulating that they were delivered!
        // self.cars.clear(); 

        Ok(())
    }


    fn calculate_cargo_weight(&self) -> u32 {
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


    pub fn assemble_train(&mut self, roundhouse: &mut Roundhouse, mission: &Mission /* <--- We take a reference to the work order */) -> Result<Train, TrainError> {

        // We extract the data we need from the mission
        let car_ids = &mission.required_cars;
        let dist = mission.distance_km;
        // We actually infer what type of engine we will need from the car_ids and their cargo weights.
        //calculate total weight of requested cars and check for missing cars before taking ownership of the engine. If any car is missing or if the total weight exceeds the engine's capacity, we can return an error without having to worry about returning the engine or any cars we might have already taken ownership of.
        let mut total_weight = 0;
        
        for id in car_ids {
            //We might get rid of this explicit check in the future and just rely on the filter_map and map combinators to do the work for us, but for now we'll keep it like this for clarity and to avoid any potential issues with ownership and borrowing when we move on to taking ownership of the cars and engine later in the function.
            match self.cars.get(id) {
                Some(car) => total_weight += car.calculate_cargo_weight(),
                None => Err(TrainError::AssemblyFailed { 
                    missing_car_ids: vec![*id], 
                    engine_returned: 0 // No engine pulled yet 
                })?
            }
        }

        // Now we have the total weight of the requested cars, we can find a suitable engine from the roundhouse.
    
        // This transforms the Option<Engine> into a Result<Engine, TrainError> on the fly!
        let engine = roundhouse.find_suitable_engine(total_weight, dist)
            .ok_or(TrainError::NoAvailableEngine)?;

        // let actual_capacity = roundhouse.stalls.get(&engine_req)
        //     .and_then(|queue| queue.front()) // Peek at reference to the next engine of the requested type
        //     .map(|engine| engine.current_capacity()) // Check its current capacity
        //     .unwrap_or(0); // If no engines of that type are available, treat as zero capacity

        // 1. Take ownership of the power

        //MOOWAHAHA! Functional programming style is all mine! (for now, with Google's Gemini's and Copilot's help...)
        let attached_cars = car_ids.iter()
            .filter_map(|id| self.cars.remove(id)) // Try to take ownership of each requested car: returns Option<TrainCar>
            .collect(); // Collect the successfully removed cars into a Vec<TrainCar>

        // Gathering the payload: We have already confirmed that all requested cars exist and that the engine can handle the weight, so now we can take ownership of the cars and move them into the train. If any car is missing at this point, it means something went wrong with our earlier checks, and we will need to roll back by returning any cars we did find and returning the engine to the roundhouse.
        // for id in &car_ids {
        //     let car = self.cars.remove(id).unwrap(); // We can safely unwrap here because we already checked for missing cars
        //     attached_cars.push(car);
        // }

        Ok(Train {
            id: self.generate_new_id(),
            engine,
            cars: attached_cars,
            distance_km: dist,
            mission_id: Some(mission.id),
        })

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

    // Notice we pass the Station's own internal roundhouse to the yard.
    pub fn dispatch_train(&mut self, mission: &Mission) -> Result<Train, TrainError> {
        println!("{BOLD}{CYAN}[{}] Orchestrating Assembly for Mission {}...{RESET}", self.name, mission.id);
        self.yard.assemble_train(&mut self.roundhouse, mission)
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



fn main() {


    let mut tidmouth = Station::new("Tidmouth");
    let mut brendam_docks = Station::new("Brendam Docks");

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


    for car in tidmouth_incoming_cars {
        let car_id = car.id;
        match tidmouth.yard.receive_car(car) {
            Ok(_) => println!("Car {} successfully received into the yard.", car_id),
            Err((homeless_car, error)) => {
                println!("Intake failed for Car {}: {:?}. Moving to purgatory.", homeless_car.id, error);
                let rejected_asset = RejectedAsset::new(homeless_car, error, 0, None); // We can fill in the timestamp and source_mission later when we implement those features.
                tidmouth.yard.purgatory.push(rejected_asset);
            }
        }
    }


    let engine4 = Engine { id: 1, engine_type: EngineType::Thomas, current_fuel: 1000.0 };
    let engine2 = Engine { id: 2, engine_type: EngineType::Thomas, current_fuel: 2000.0 };
    let engine3 = Engine { id: 3, engine_type: EngineType::Percy, current_fuel: 500.0 };
    let engine1 = Engine { id: 4, engine_type: EngineType::Diesel, current_fuel: 500.0 };
    let engine5 = Engine { id: 5, engine_type: EngineType::Gordon, current_fuel: 5000.0 };



    //Switched it up to intentionally block a full-fuel Thomas with a half-fuel Thomas to test the find_suitable_engine method. Since the half_fuel Thomas is technically the correct type for the mission, but doesn't have the fuel to complete it, we should see the roundhouse skip it and move on to the next option in the roster, which is the Gordon.
    tidmouth.roundhouse.house(engine1);
    tidmouth.roundhouse.house(engine4);
    tidmouth.roundhouse.house(engine3);
    tidmouth.roundhouse.house(engine2);
    tidmouth.roundhouse.house(engine5);

    
    tidmouth.yard.print_report(&tidmouth.roundhouse);
    brendam_docks.yard.print_report(&brendam_docks.roundhouse);

    
    let mission1: Mission = Mission { id: 1, destination: String::from("Brendam Docks"), required_cars: vec![2, 4, 6], distance_km: 250 };


    // match tidmouth_yard.assemble_train(&mut tidmouth_roundhouse, engine_req, car_ids) {
    //     Ok(mut new_train) => {
    //         println!("{GREEN}Success! Train {} assembled with Engine {}.{RESET}", new_train.id, new_train.engine.id);
    //         new_train.dispatch().ok();
    //         yard.trains.push(new_train); // Add to active missions
    //     },
    //     Err(e) => println!("{RED}Assembly Failed: {:?}{RESET}", e),
    // }


    match tidmouth.dispatch_train(&mission1) {
        Ok(mut new_train) => {
            if let Ok(_) = new_train.dispatch() {
                println!("{GREEN}Mission {} completed successfully! Train {} has arrived at {}.{RESET}", mission1.id, new_train.id, mission1.destination);
                brendam_docks.receive_train(new_train); // The train arrives at the destination station, which triggers the disassembly process.
            }
        }
        Err(e) => println!("{RED}Mission to {} from {} failed to dispatch: {:?}{RESET}", mission1.destination, tidmouth.name, e),
    }


    tidmouth.yard.print_report(&tidmouth.roundhouse);

    // let completed_train = tidmouth.yard.trains.pop().unwrap(); // We can safely unwrap here because we know we just added a train to the active missions
    // tidmouth.yard.disassemble_train(completed_train, &mut roundhouse);

    
    brendam_docks.yard.print_report(&brendam_docks.roundhouse);


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
            EngineType::Diesel => 5.0, // Devious, but extremely efficient
            EngineType::Percy => 3.0, //  Smart and efficient, but not the strongest
            EngineType::Thomas => 2.5, // Classic, Jack of all trades
            EngineType::Gordon => 1.8, // Powerful, but a gas guzzler
        }
    }
}
