use std::{clone, error};
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
    fuel_level: FuelLevel,
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

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)] // This allows us to easily create copies of EngineType values, which is useful for passing them around without losing ownership.
enum EngineType {
    Diesel,
    Thomas,
    Percy,
    Gordon,
}

#[derive(Debug)]
enum FuelLevel {
    Full,
    Half,
    Low,
}


#[derive(Debug)]
enum TrainError {
    EngineOverheat,
    DieselInTheStation,
    LowFuel,
    ContrabandOnBoard(String),
    NoCargoOrPassengers,
    DuplicateId(u32),
    NoAvailableEngine(EngineType),
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
        self.fuel_level = FuelLevel::Full;
        println!("Engine {} has been rehabilitated and refueled!", self.id);
    }

    fn refuel(&mut self) {
        println!("Refueling Engine {}...", self.id);
        // Logic to refuel the engine
        // For demonstration, we'll just set the fuel level to Full
        self.fuel_level = FuelLevel::Full;
        println!("Engine {} is now fully refueled!", self.id);
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
        match self.engine.fuel_level {
            FuelLevel::Full=> Ok(String::from("Fuel level is sufficient!")),
            _ => Err(TrainError::LowFuel),
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
    

    pub fn print_report(&self) {
        println!("\n{BOLD}{CYAN}┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓{RESET}");
        println!("{BOLD}{CYAN}┃              SODOR RAILWAY: YARD REPORT               ┃{RESET}");
        println!("{BOLD}{CYAN}┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛{RESET}");

        // 1. THE MAIN YARD (The Lockers)
        println!("  {BOLD}MAIN YARD LOCKERS ({}/{} capacity used){RESET}", self.cars.len(), 100); // We can make capacity a real variable later
        if self.cars.is_empty() {
            println!("    (No cars currently parked)");
        } else {
            for (id, car) in &self.cars {
                let cargo_desc = match &car.cargo {
                    Some(c) => format!("{} ({}kg)", c.item, c.actual_weight),
                    None => "Empty".to_string(),
                };
                let pax = car.passenger.as_deref().unwrap_or("None");
                println!("    {GREEN}[ID: {:02}]{RESET} | Pax: {:<10} | Cargo: {}", id, pax, cargo_desc);
            }
        }

        // 2. THE PURGATORY (The Stray Track)
        println!("\n  {BOLD}{RED}PURGATORY SIDING (Stray/Invalid Cars){RESET}");
        if self.purgatory.is_empty() {
            println!("    (Clear - All cars accounted for)");
        } else {
            for car in &self.purgatory {
                println!("    {RED}⚠️ [ID: {:02}] | REJECTED | Pax: {} | Check Manifest immediately!{RESET}", car.id, car.passenger.as_deref().unwrap_or("Unknown"));
            }
        }

        // 3. THE ROUNDHOUSE (Coming Soon)
        println!("\n  ROUNDHOUSE (Engines)");
        println!("    [ TODO: Implement Engine Sorting ]");
        
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
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





    pub fn assemble_train(&mut self, roundhouse: &mut Roundhouse, engine_req: EngineType, car_ids: Vec<u32>) -> Result<Train, TrainError> {
        // 1. Acquire Power
        let engine = roundhouse.dispatch(engine_req)
            .ok_or(TrainError::NoAvailableEngine(engine_req))?;

        // 2. Acquire Payload
        let mut attached_cars = Vec::new();
        for id in car_ids {
            if let Some(car) = self.cars.remove(&id) {
                attached_cars.push(car);
            }
        }

        // 3. Create the Assembly (The Train)
        Ok(Train {
            id: self.generate_new_id(), // We'll need a way to track these
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
}



fn main() {


    let mut yard: Railyard = Railyard::new();
    let mut roundhouse: Roundhouse = Roundhouse::new();


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


    if let Err((homeless_car, error)) = yard.receive_car(carriage) {
        println!("Intake failed for Car {}: {:?}", homeless_car.id, error);
        yard.purgatory.push(homeless_car);
    }
    if let Err((homeless_car, error)) = yard.receive_car(dining_car) {
        println!("Intake failed for Car {}: {:?}", homeless_car.id, error);
        yard.purgatory.push(homeless_car);
    }
    if let Err((homeless_car, error)) = yard.receive_car(boxcar1) {
        println!("Intake failed for Car {}: {:?}", homeless_car.id, error);
        yard.purgatory.push(homeless_car);
    }
    if let Err((homeless_car, error)) = yard.receive_car(boxcar2) {
        println!("Intake failed for Car {}: {:?}", homeless_car.id, error);
        yard.purgatory.push(homeless_car);
    }
    if let Err((homeless_car, error)) = yard.receive_car(boxcar3) {
        println!("Intake failed for Car {}: {:?}", homeless_car.id, error);
        yard.purgatory.push(homeless_car);
    }
    if let Err((homeless_car, error)) = yard.receive_car(boxcar4) {
        println!("Intake failed for Car {}: {:?}", homeless_car.id, error);
        yard.purgatory.push(homeless_car);
    }
    if let Err((homeless_car, error)) = yard.receive_car(caboose) {
        println!("Intake failed for Car {}: {:?}", homeless_car.id, error);
        yard.purgatory.push(homeless_car);
    }

yard.print_report();


    let engine1 = Engine { id: 1, engine_type: EngineType::Diesel, fuel_level: FuelLevel::Low };
    let engine2 = Engine { id: 2, engine_type: EngineType::Thomas, fuel_level: FuelLevel::Full };
    let engine3 = Engine { id: 3, engine_type: EngineType::Percy, fuel_level: FuelLevel::Half };
    let engine4 = Engine { id: 4, engine_type: EngineType::Thomas, fuel_level: FuelLevel::Half };
    let engine5 = Engine { id: 5, engine_type: EngineType::Gordon, fuel_level: FuelLevel::Full };



    
    roundhouse.house(engine1);
    roundhouse.house(engine2);
    roundhouse.house(engine3);
    roundhouse.house(engine4);
    roundhouse.house(engine5);


    println!("{BOLD}{YELLOW}--- MISSION DISPATCH: REQUESTING A GORDON ---{RESET}");
    let car_ids = vec![1, 2, 3]; // Requesting the specific cars we just added
    
    match yard.assemble_train(&mut roundhouse, EngineType::Gordon, car_ids) {
        Ok(mut new_train) => {
            println!("{GREEN}Success! Train {} assembled with Engine {}.{RESET}", new_train.id, new_train.engine.id);
            new_train.dispatch().ok();
            yard.trains.push(new_train); // Add to active missions
        },
        Err(e) => println!("{RED}Assembly Failed: {:?}{RESET}", e),
    }

    yard.print_report();





}



fn describe_personality(engine: &EngineType) -> String{
    match engine {
        EngineType::Thomas => String::from("Thomas is a friendly and helpful engine, always ready to lend a hand and make friends. Thomas is the most popular."),
        EngineType::Percy => String::from("Percy is a brave and intuitive little engine that doesn't always think things through, but always does his best. Percy is the most adventurous."),
        EngineType::Gordon => String::from("Gordon is proud and doesn't like to admit when he's wrong, but he cares about his friends. Gordon is the strongest."),
        EngineType::Diesel => String::from("Diesel is a troublemaker."),
    }
}
