



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
    manifest_weight: u32,
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

struct Train{
    id: u32,
    cars: Vec<TrainCar>,
    engine: Engine, // Ownership! The Engine is PHYSICALLY in the Train now.
}



struct Railyard {
    trains: Vec<Train>,
    cars: HashMap<u32, TrainCar>,
    next_train_id: u32,
    purgatory: Vec<TrainCar>
    //cargo: Vec<Cargo>,

}


// Assuming EngineType and Engine are defined
struct Roundhouse {
    stalls: HashMap<EngineType, VecDeque<Engine>>,
}


struct Mission {
    id: u32,
    destination: String,
    required_cars: Vec<u32>,
    distance_km: u32,
}


#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)] // This allows us to easily create copies of EngineType values, which is useful for passing them around without losing ownership.
enum EngineType {
    Diesel,
    Thomas,
    Percy,
    Gordon,
}

// #[derive(Debug)]
// enum FuelLevel {
//     Full,
//     Half,
//     Low,
// }


#[derive(Debug)]
enum TrainError {
    EngineOverheat,
    DieselInTheStation,
    LowFuel,
    ContrabandOnBoard(String),
    NoCargoOrPassengers,
    DuplicateId(u32),
    // ... our existing variants ...
    NoAvailableEngine(EngineType),
    AssemblyFailed {
        missing_car_ids: Vec<u32>,
        engine_returned: u32,
    },
    EngineCapacityExceeded {
        required: u32,
        capacity: u32,
    },
}



impl Cargo {
    fn check_contraband(&self) -> Result<String, TrainError> {
        match self.manifest_weight == self.actual_weight{
            true => Ok(format!("Cargo '{}' is clear of contraband.", self.item)),
            false => match &self.contraband {
                Some(item) => {
                    println!("Contraband found on cargo '{}'!", item);
                    Err(TrainError::ContrabandOnBoard(format!("Contraband detected: {}!", item)))
                },
                None => Ok(format!("Cargo '{}' has a weight discrepancy but no contraband detected.", self.item)),
            }
        }
    }
}



impl TrainCar {
    fn check_passenger(&self) {
        match &self.passenger {
            Some(name) => println!("{} is aboard!", name),
            None => println!("Ain't nobody on this train car!"),
        }
    }

    /* */
    fn check_contraband(&self) -> Result<String, TrainError> {
        if let Some(cargo) = &self.cargo {
            cargo.check_contraband()
        } else {
            Ok(String::from("No cargo on board, so no contraband!"))
        }
    }

    fn check_freight(&self) -> Result<String, TrainError> {
        match (&self.cargo, &self.passenger) {
            (None, None) => Err(TrainError::NoCargoOrPassengers),
            (Some(cargo), None) => Ok(format!("Cargo on board: {:?}", cargo)),
            (None, Some(passenger)) => Ok(format!("Passenger aboard: {}", passenger)),
            (Some(cargo), Some(passenger)) => Ok(format!("Cargo on board: {:?}. Passenger aboard: {}", cargo, passenger)),
        }
        
    }

    fn calculate_cargo_weight(&self) -> u32 {
        self.cargo
            .as_ref()
            .map(|c| c.actual_weight)
            .unwrap_or(0)
    }

    // fn calculate_cargo_weight(&self) -> u32 {
    //     match &self.cargo {
    //         Some(cargo) => cargo.actual_weight,
    //         None => 0,
    //     }
    // }

    fn prepare_for_departure(&self) -> Result<String, TrainError> {
        //How come we no longer reference self.start_engine() with &self.start_engine()? Is it because we are already borrowing self in the method signature, so we can call self.start_engine() directly without needing to borrow it again? Yes, that's correct! Since the method signature already borrows self as an immutable reference (&self), we can call other methods on self directly without needing to borrow it again. The Rust compiler understands that we are working with a borrowed reference to self and allows us to call methods on it without needing to explicitly borrow it again. So in this case, we can simply call self.start_engine() without needing to use &self.start_engine(). The compiler will handle the borrowing for us and ensure that we are using the borrowed reference correctly.
         let freight_status = self.check_freight()?;
         // Where does OK(String::from("The train is ready for departure!")) come from? Is it just a way to return a successful result from the function, indicating that the train is ready for departure? Yes, that's correct! The Ok(String::from("The train is ready for departure!")) is a way to return a successful result from the prepare_for_departure() function. It indicates that the engine started successfully and the train is ready for departure. The Ok variant of the Result type is used to represent a successful outcome, while the Err variant is used to represent an error. In this case, if the engine starts successfully, we return an Ok value with a message indicating that the train is ready for departure. If there was an error starting the engine (like if it's a Diesel), we would return an Err value with the appropriate TrainError.
         let contraband_status = self.check_contraband()?;
         
         Ok(format!("Preparing Car {} for departure. Freight Status: {}. Contraband Status: {}", self.id, freight_status, contraband_status))
    }

}


impl Engine {
    fn rehabilitate(&mut self) {
        println!("Rehabilitating Engine {}...", self.id);
        // Logic to rehabilitate the engine, e.g., fixing mechanical issues
        // For demonstration, we'll just print a message and set fuel level to Full
        self.current_fuel = self.engine_type.max_fuel_capacity(); // Refill the fuel as part of rehabilitation
        println!("Engine {} has been rehabilitated and refueled!", self.id);
    }

    fn refuel(&mut self) {
        println!("Refueling Engine {}...", self.id);
        // Logic to refuel the engine
        // For demonstration, we'll just set the fuel level to Full
        self.current_fuel = self.engine_type.max_fuel_capacity();
        println!("Engine {} is now fully refueled!", self.id);
    }

     pub fn current_capacity(&self) -> u32 {
        let max_capacity = self.engine_type.max_capacity();
        let fuel_multiplier = self.current_fuel / self.engine_type.max_fuel_capacity(); // This gives us a value between 0.0 and 1.0 representing how full the engine is
        (max_capacity as f32 * fuel_multiplier) as u32
    }

    pub fn can_complete_mission(&self, total_weight: u32, distance_km: u32) -> bool {
        // match ((total_weight * distance_km) / (self.engine_type.fuel_efficiency() as u32)) <= self.current_capacity() {
        //     true => println!("{GREEN}Engine {} can complete the mission!{RESET}", self.id),
        //     false => println!("{RED}Engine {} cannot complete the mission due to weight and distance requirements.{RESET}", self.id),
        // }

        let work_required = (total_weight as f32) * (distance_km as f32);
        let fuel_quotient: f32 = self.engine_type.fuel_efficiency() * 1000.0; // This scaling factor is arbitrary and can be adjusted based on how you want to balance the formula
        let fuel_required = work_required / fuel_quotient;
        if fuel_required > self.current_fuel {
            println!("{RED}Mission Impossible: Even at current fuel levels, Engine {} cannot complete the mission due to weight and distance requirements.{RESET}", self.id);
            false
        } else {
            println!("{GREEN}Mission Possible: Engine {} can complete the mission with current fuel levels!{RESET}", self.id);
            true
        }

        // // 1. Get the engine's efficiency from its EngineType
        // let efficiency = self.engine_type.fuel_efficiency();
        // // 2. Calculate the "PercentageRequired" using the formula and scaling factor above
        // let work_required = total_weight as f32 * distance_km as f32;
        // let fuel_required = work_required / (efficiency * 1000.0);
        // // (Don't forget to cast your u32 variables using `as f32` before dividing!)
        // if fuel_required > self.engine_type.max_fuel_capacity() {
        //     println!("{RED}Mission Impossible: Even at full fuel, Engine {} cannot complete the mission due to weight and distance requirements.{RESET}", self.id);
        //     return false;
        // }
        // // 3. Get the engine's current available fuel percentage
        // else {
        //     true;
        // }
        // // 4. Return true if available >= required, else false
        
        // // Placeholder return so it compiles while you think
        // false 
    }
}


impl Train {
    
    fn start_engine(&self) -> Result<String, TrainError> {
        match self.engine.engine_type {
            EngineType::Diesel => Err(TrainError::DieselInTheStation),
            _ => Ok(String::from("The engine starts successfully!")),
        }
    }

    fn check_fuel(&self) -> Result<String, TrainError> {
        if self.engine.current_fuel < self.engine.engine_type.max_fuel_capacity() * 0.5 {
            Err(TrainError::LowFuel)
        } else {
            Ok(format!("Fuel level is sufficient at {} units.", self.engine.current_fuel))
        }
    }


    
    fn prepare_for_departure(&self) -> Result<String, TrainError> {
        //How come we no longer reference self.start_engine() with &self.start_engine()? Is it because we are already borrowing self in the method signature, so we can call self.start_engine() directly without needing to borrow it again? Yes, that's correct! Since the method signature already borrows self as an immutable reference (&self), we can call other methods on self directly without needing to borrow it again. The Rust compiler understands that we are working with a borrowed reference to self and allows us to call methods on it without needing to explicitly borrow it again. So in this case, we can simply call self.start_engine() without needing to use &self.start_engine(). The compiler will handle the borrowing for us and ensure that we are using the borrowed reference correctly.
         let engine_status = self.start_engine()?;
         
         let fuel_status = self.check_fuel()?;
         
         Ok(format!("Departure Status: {}, Fuel Status: {:?}", engine_status, fuel_status))
    }



    pub fn purge_rejected_cars(&mut self) -> Vec<TrainCar> {
    let mut rejected_cars = Vec::new();
    let mut good_cars = Vec::new();

    // The Train surgically removes its own cars
    for car in self.cars.drain(..) {
        if car.prepare_for_departure().is_ok() {
            good_cars.push(car);
        } else {
            rejected_cars.push(car);
        }
    }

    // Re-assign the good cars to the train
    self.cars = good_cars;

    // Hand the duds back to the caller
    rejected_cars
}

    fn dispatch(&self) -> Result<Vec<&TrainCar>, TrainError> {
        
        match self.prepare_for_departure() {
            Ok(status) => println!("Train {}: {}", self.id, status),
            Err(e) => {
                println!("Train {} cannot depart: {:?}", self.id, e);
                return Err(e);
            }
        }

        for car in &self.cars {
           
            match car.prepare_for_departure() {
                Ok(msg) => println!("Train Car {}: {}", car.id, msg),
                Err(e) => {
                    println!("Train Car {}: Error preparing for departure: {:?}", car.id, e);
                    println!("--- Dispatcher: Skipping car {} and moving to next... ---", car.id);
                }
            }
        }

        let ok_engine_line: Vec<&TrainCar> = self.cars.iter()// // 1. Start the conveyor belt
        .filter(|&car| car.prepare_for_departure().is_ok()) // 2. "Filter" out the Diesels and Low_Fuel cars
        .collect(); // 3. Put cars that did not return an error into a new Box (Vec)

    
        Ok(ok_engine_line)
            
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
                println!("    {RED}⚠️ [CAR ID: {:02}] | REJECTED | Check Manifest immediately!{RESET}", car.id);
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






    fn house(&mut self, train: Train) {
        //println!("\n\n  house called with Train ID {}. Adding to the yard...", train.id);
        self.trains.push(train);
    }


    
    fn add_car(&mut self, car: TrainCar) {
        //println!("\n\n  add_car called with Car ID {}. Adding to the yard without duplicate check...", car.id);
        self.cars.insert(car.id, car);
    }

    pub fn receive_car(&mut self, car: TrainCar) -> Result<(), (TrainCar, TrainError)> {
        // 1. Explicit Check: No silent side-effects
        if self.cars.contains_key(&car.id) {
            println!("{RED}Railyard Error: Car ID {} is a duplicate!{RESET}", car.id);
            let car_id = car.id;
            return Err((car, TrainError::DuplicateId(car_id)));
        }

        // 2. Success: The state change is clear
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

            self.receive_car(car);

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
        
        // // Keeping the following for now as a reference for how we can use iterators and combinators
        // // The "Ghost Logic" without the "Ghost Struct"
        // let total_weight: u32 = car_ids.iter()
        //     .filter_map(|id| self.cars.get(id)) // Search for the car: returns Option<&TrainCar>
        //     .map(|car| car.calculate_cargo_weight()) // Call the weight method on the reference
        //     .sum();

        for id in car_ids {

            // if let Some(car) = self.cars.get(id) { // .get() returns a reference &TrainCar
            //     total_weight += car.calculate_cargo_weight();
            // } else {
            //     return Err(TrainError::AssemblyFailed {
            //         missing_car_ids: vec![*id],
            //         engine_returned: 0, // No engine pulled yet,
            //     });
            // }

            match self.cars.get(id) {
                Some(car) => total_weight += car.calculate_cargo_weight(),
                None => Err(TrainError::AssemblyFailed { 
                    missing_car_ids: vec![*id], 
                    engine_returned: 0 // No engine pulled yet 
                })?
            }
        }

        // let actual_capacity = roundhouse.stalls.get(&engine_req)
        //     .and_then(|queue| queue.front()) // Peek at reference to the next engine of the requested type
        //     .map(|engine| engine.current_capacity()) // Check its current capacity
        //     .unwrap_or(0); // If no engines of that type are available, treat as zero capacity

        // 1. Take ownership of the power
        


        // let can_proceed = actual_capacity >= total_weight;
       
        // if !can_proceed {
        //     println!("{RED}Assembly Failed: No available engine of type {:?} can handle the total cargo weight of {}. Returning Engine to Roundhouse.{RESET}", engine_req, total_weight);
        //     return Err(TrainError::EngineCapacityExceeded { required: total_weight, capacity: actual_capacity });
        // }

        // let engine = roundhouse.dispatch(engine_req).unwrap();




        // actual_capacity = engine.current_capacity();

        // if total_weight > actual_capacity {
        //     println!("{RED}Assembly Failed: Total cargo weight {} exceeds Engine {}'s capacity of {}. Returning Engine to Roundhouse.{RESET}", total_weight, engine.id, actual_capacity);
        //     roundhouse.house(engine); // Return the engine immediately
        //     return Err(TrainError::EngineCapacityExceeded { required: total_weight, capacity: actual_capacity });
        // }


        //MOOWAHAHA! Functional programming style are belong to me! (for now, with Google's Gemini's and Copilot's help...)
        let attached_cars = car_ids.iter()
            .filter_map(|id| self.cars.remove(id)) // Try to take ownership of each requested car: returns Option<TrainCar>
            .collect(); // Collect the successfully removed cars into a Vec<TrainCar>

        // Gathering the payload: We have already confirmed that all requested cars exist and that the engine can handle the weight, so now we can take ownership of the cars and move them into the train. If any car is missing at this point, it means something went wrong with our earlier checks, and we will need to roll back by returning any cars we did find and returning the engine to the roundhouse.
        // for id in &car_ids {
        //     let car = self.cars.remove(id).unwrap(); // We can safely unwrap here because we already checked for missing cars
        //     attached_cars.push(car);
        // }

        let engine = roundhouse.find_suitable_engine(total_weight, dist)
            .ok_or(TrainError::NoAvailableEngine(EngineType::Diesel))?; // We can specify a more detailed error here if we want, but for now we'll just say no suitable engine available.

        Ok(Train {
            id: self.generate_new_id(),
            engine,
            cars: attached_cars,
        })

    }





     fn dispatch_trains(&self) {
        for train in &self.trains {
            match train.dispatch() {
                Ok(ok_cars) => println!("Train {} is ready for departure with {} cars!", train.id, ok_cars.len()),
                Err(e) => println!("Train {} cannot depart: {:?}", train.id, e),
            }
        }
    }




    pub fn service_train(&mut self, mut train: Train) -> Train {
        println!("Servicing Train {}...", train.id);
        train.engine.rehabilitate();    // At some point, we might want to have different levels of service that do different things to the engine and cars, but for now we'll just do a full rehab and refuel.
        train.engine.refuel();  // Eventually, we could have different levels of service that do different things to the engine and cars, but for now we'll just do a full rehab and refuel.
        
        let mut ok_cars: Vec<TrainCar> = Vec::new();

        for car in train.cars.drain(..) {
            match car.prepare_for_departure() {
                Ok(msg) => {
                    println!("Train Car {} is ready for departure: {}", car.id, msg);
                    ok_cars.push(car);
                }
                Err(e) => {
                    println!("Train Car {} cannot depart: {:?}. Pushing to Railyard.", car.id, e);

                    self.receive_car(car);
                    
                }
            }
        }
        train.cars = ok_cars;
        train
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
                    
                    // // Peek at the front engine
                    // if let Some(engine) = queue.front() {
                        
                    //     // Ask the engine if it has the fuel for the distance
                    //     if engine.can_complete_mission(total_weight, distance_km) {
                            
                    //         // WE HAVE A WINNER. Pop it from the stall and return it.
                    //         return queue.pop_front();
                    //     }
                    // }
                    // Assume we are already inside the `if let Some(queue) = self.stalls.get_mut(&etype)` block

                    let mut found_index = None; // Our clipboard

                    // .iter().enumerate() gives us both the position (0, 1, 2...) and the engine
                    for (index, engine) in queue.iter().enumerate() {
                        if engine.can_complete_mission(total_weight, distance_km) {
                            found_index = Some(index); // Write down the number!
                            break; // Stop looking! We found our fully-fueled Thomas.
                        }
                    }

                    // If we wrote a number on our clipboard, remove that specific engine
                    if let Some(index) = found_index {
                        return queue.remove(index); // This physically pulls him out of the line!
                    }
                }

                // // 1. Find the position of the first capable engine
                // let winner_index = queue.iter().position(|engine| {
                //     engine.can_complete_mission(total_weight, distance_km)
                // });

                // // 2. Chain it using the `.and_then()` you love!
                // // If position returned Some(index), and_then passes that index into queue.remove()
                // return winner_index.and_then(|index| queue.remove(index));
            }
        }
        
        // If we loop through the whole roster and find nothing, return None.
        None
    }
}


//testing testing. what happened to inline suggestions? HAHA! They're back. Is this cheating? ::: Maybe. But it's also a great way to quickly iterate on code without needing to worry about the borrow checker until we have a working version of the logic. Once we have the logic down, we can go back and clean up the code and make it more idiomatic. So in that sense, it's a useful tool for learning and prototyping. But it's also important to eventually understand how to write code that works with the borrow checker and ownership system in Rust, so I would recommend using inline suggestions as a way to quickly iterate on code, but also taking the time to learn how to write code that works with Rust's ownership system without relying on inline suggestions.
fn main() {


    let mut yard: Railyard = Railyard::new();
    let mut roundhouse: Roundhouse = Roundhouse::new();
    let mission1: Mission = Mission { id: 1, destination: String::from("Brendam Docks"), required_cars: vec![2, 4, 6], distance_km: 500 };


    let cargo1 = Cargo { item: String::from("bananas"), manifest_weight: 1000, actual_weight: 1000, contraband: None };
    let cargo2 = Cargo { item: String::from("crates of oranges"), manifest_weight: 1000, actual_weight: 1005, contraband: Some(String::from("Stylish TUMI Briefcase")) };
    let cargo3 = Cargo { item: String::from("Redacted Documents"), manifest_weight: 2000, actual_weight: 11001, contraband: Some(String::from("The Service Weapon")) };
    let cargo4 = Cargo { item: String::from("Various Crafting Ingredients"), manifest_weight: 1500, actual_weight: 1500, contraband: None };
    let cargo5 = Cargo { item: String::from("Scrap Metal"), manifest_weight: 10000, actual_weight: 10075, contraband: Some(String::from("Excessively Heavy Fire Extinguisher")) };
    let cargo6 = Cargo { item: String::from("pallets of electronics"), manifest_weight: 3000, actual_weight: 3000, contraband: None };
    let cargo7 = Cargo { item: String::from("Redacted Documents"), manifest_weight: 2000, actual_weight: 11001, contraband: Some(String::from("The Service Weapon")) };


    

    let carriage = TrainCar { id:1, cargo: Some(cargo2), passenger: Some(String::from("Lemon:"))};
    let dining_car = TrainCar { id:2, cargo: Some(cargo1), passenger: Some(String::from("Ladybug"))};
    let boxcar1 = TrainCar { id:3, cargo: Some(cargo5), passenger: Some(String::from("Blazkowicz")),};
    let boxcar2 = TrainCar { id:4, cargo: Some(cargo6), passenger: Some(String::from("Tangerine")),};
    let boxcar3 = TrainCar { id:5, cargo: Some(cargo3), passenger: Some(String::from("Faden")),};
    let boxcar4 = TrainCar { id:5, cargo: Some(cargo7), passenger: Some(String::from("Faden")),};
    let caboose = TrainCar { id:6, cargo: Some(cargo4), passenger: Some(String::from("Artyom"))};


    let incoming_cars = vec![carriage, dining_car, boxcar1, boxcar2, boxcar3, boxcar4, caboose];


    for car in incoming_cars {
        let car_id = car.id;
        match yard.receive_car(car) {
            Ok(_) => println!("Car {} successfully received into the yard.", car_id),
            Err((homeless_car, error)) => {
                println!("Intake failed for Car {}: {:?}. Moving to purgatory.", homeless_car.id, error);
                yard.purgatory.push(homeless_car);
            }
        }
    }

    
    // for car in incoming_cars {
    //     if let Err((homeless_car, error)) = yard.receive_car(car) {
    //         println!("Intake failed for Car {}: {:?}", homeless_car.id, error);
    //         yard.purgatory.push(homeless_car);
    //     }
    // }

    // if let Err((homeless_car, error)) = yard.receive_car(carriage) {
    //     println!("Intake failed for Car {}: {:?}", homeless_car.id, error);
    //     yard.purgatory.push(homeless_car);
    // }

yard.print_report(&roundhouse);


    let engine1 = Engine { id: 1, engine_type: EngineType::Diesel, current_fuel: 500.0 };
    let engine2 = Engine { id: 2, engine_type: EngineType::Thomas, current_fuel: 2000.0 };
    let engine3 = Engine { id: 3, engine_type: EngineType::Percy, current_fuel: 500.0 };
    let engine4 = Engine { id: 4, engine_type: EngineType::Thomas, current_fuel: 1000.0 };
    let engine5 = Engine { id: 5, engine_type: EngineType::Gordon, current_fuel: 5000.0 };



    //Switched it up to intentionally block a full-fuel Thomas with a half-fuel Thomas to test the find_suitable_engine method. Since the half_fuel Thomas is technically the correct type for the mission, but doesn't have the fuel to complete it, we should see the roundhouse skip it and move on to the next option in the roster, which is the Gordon.
    roundhouse.house(engine1);
    roundhouse.house(engine4);
    roundhouse.house(engine3);
    roundhouse.house(engine2);
    roundhouse.house(engine5);


    // println!("{BOLD}{YELLOW}--- MISSION DISPATCH: REQUESTING A GORDON ---{RESET}");
    // let car_ids = vec![1, 2, 3]; // Requesting the specific cars we just added
    // //let car_ids = vec![1, 2, 3, 7]; // Intentional Failure Check
    // let engine_req: EngineType = EngineType::Gordon; // passes (just about everything)
    // //let engine_req: EngineType = EngineType::Percy; // intentional failure check
    
    // match yard.assemble_train(&mut roundhouse, engine_req, car_ids) {
    //     Ok(mut new_train) => {
    //         println!("{GREEN}Success! Train {} assembled with Engine {}.{RESET}", new_train.id, new_train.engine.id);
    //         new_train.dispatch().ok();
    //         yard.trains.push(new_train); // Add to active missions
    //     },
    //     Err(e) => println!("{RED}Assembly Failed: {:?}{RESET}", e),
    // }

    match yard.assemble_train(&mut roundhouse, &mission1) {
        Ok(mut new_train) => {
            println!("{GREEN}Success! Train {} assembled with Engine {}.{RESET}", new_train.id, new_train.engine.id);
            new_train.dispatch().ok();
            yard.trains.push(new_train); // Add to active missions
        },
        Err(e) => println!("{RED}Assembly Failed: {:?}{RESET}", e),
    }

    yard.print_report(&roundhouse);


    


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

    fn describe_personality(&self) -> String{
        match self {
            EngineType::Thomas => String::from("Thomas is a friendly and helpful engine, always ready to lend a hand and make friends. Thomas is the most popular."),
            EngineType::Percy => String::from("Percy is a brave and intuitive little engine that doesn't always think things through, but always does his best. Percy is the most adventurous."),
            EngineType::Gordon => String::from("Gordon is proud and doesn't like to admit when he's wrong, but he cares about his friends. Gordon is the strongest."),
            EngineType::Diesel => String::from("Diesel is a troublemaker."),
        }
    }
}

// impl FuelLevel {
//     fn capacity_multiplier(&self) -> f32 {
//         match self {
//             FuelLevel::Ten => 1.00,
//             FuelLevel::Nine => 0.90,
//             FuelLevel::Eight => 0.80,
//             FuelLevel::Seven => 0.70,
//             FuelLevel::Six => 0.60,
//             FuelLevel::Five => 0.50,
//             FuelLevel::Four => 0.40,
//             FuelLevel::Three => 0.30,
//             FuelLevel::Two => 0.20,
//             FuelLevel::One => 0.10,
//             FuelLevel::Empty => 0.00,
//         }
//     }
// }