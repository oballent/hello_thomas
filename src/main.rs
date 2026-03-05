use std::clone;
use std::collections::HashMap;

// This program demonstrates the concept of mutable references in Rust using a simple example of train engines and their personalities.


#[derive(Debug)]
struct Cargo{
    item: String,
    manifest_weight: u32,
    actual_weight: u32,
    contraband: Option<String>,
}


struct TrainCar {
    id: u32,
    cargo: Option<Cargo>,
    passenger: Option<String>,
}

struct Train{
    id: u32,
    cars: Vec<TrainCar>,
    engine: EngineType,
    fuel_level: FuelLevel,
}

//#[derive(Clone, Copy)] // This allows us to easily create copies of EngineType values, which is useful for passing them around without losing ownership.
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


/*
    fn check_contraband(&self) -> Result<String, TrainError> {
        match &self.cargo.check_contraband() {
            Ok(status) => Ok(status),
            Err(e) => Err(e),
        }
    }
*/
    
    fn prepare_for_departure(&self) -> Result<String, TrainError> {
        //How come we no longer reference self.start_engine() with &self.start_engine()? Is it because we are already borrowing self in the method signature, so we can call self.start_engine() directly without needing to borrow it again? Yes, that's correct! Since the method signature already borrows self as an immutable reference (&self), we can call other methods on self directly without needing to borrow it again. The Rust compiler understands that we are working with a borrowed reference to self and allows us to call methods on it without needing to explicitly borrow it again. So in this case, we can simply call self.start_engine() without needing to use &self.start_engine(). The compiler will handle the borrowing for us and ensure that we are using the borrowed reference correctly.
         let freight_status = self.check_freight()?;
         // Where does OK(String::from("The train is ready for departure!")) come from? Is it just a way to return a successful result from the function, indicating that the train is ready for departure? Yes, that's correct! The Ok(String::from("The train is ready for departure!")) is a way to return a successful result from the prepare_for_departure() function. It indicates that the engine started successfully and the train is ready for departure. The Ok variant of the Result type is used to represent a successful outcome, while the Err variant is used to represent an error. In this case, if the engine starts successfully, we return an Ok value with a message indicating that the train is ready for departure. If there was an error starting the engine (like if it's a Diesel), we would return an Err value with the appropriate TrainError.
         let contraband_status = self.check_contraband()?;
         
         Ok(format!("Preparing Car {} for departure. Freight Status: {}. Contraband Status: {}", self.id, freight_status, contraband_status))
    }

}


impl Train {
    
    fn start_engine(&self) -> Result<String, TrainError> {
        match self.engine {
            EngineType::Diesel => Err(TrainError::DieselInTheStation),
            _ => Ok(String::from("The engine starts successfully!")),
        }
    }

    fn check_fuel(&self) -> Result<String, TrainError> {
        match self.fuel_level {
            FuelLevel::Low => Err(TrainError::LowFuel),
            _ => Ok(String::from("Fuel level is sufficient!")),
        }
    }

    fn rehabilitate(&mut self) {
        println!("Rehabilitating the train car's engine...");
        self.engine = EngineType::Thomas;
    }

    fn refuel(&mut self) {
        println!("Refueling the train car...");
        self.fuel_level = FuelLevel::Full;
    }


    
    fn prepare_for_departure(&self) -> Result<String, TrainError> {
        //How come we no longer reference self.start_engine() with &self.start_engine()? Is it because we are already borrowing self in the method signature, so we can call self.start_engine() directly without needing to borrow it again? Yes, that's correct! Since the method signature already borrows self as an immutable reference (&self), we can call other methods on self directly without needing to borrow it again. The Rust compiler understands that we are working with a borrowed reference to self and allows us to call methods on it without needing to explicitly borrow it again. So in this case, we can simply call self.start_engine() without needing to use &self.start_engine(). The compiler will handle the borrowing for us and ensure that we are using the borrowed reference correctly.
         let engine_status = self.start_engine()?;
         // Where does OK(String::from("The train is ready for departure!")) come from? Is it just a way to return a successful result from the function, indicating that the train is ready for departure? Yes, that's correct! The Ok(String::from("The train is ready for departure!")) is a way to return a successful result from the prepare_for_departure() function. It indicates that the engine started successfully and the train is ready for departure. The Ok variant of the Result type is used to represent a successful outcome, while the Err variant is used to represent an error. In this case, if the engine starts successfully, we return an Ok value with a message indicating that the train is ready for departure. If there was an error starting the engine (like if it's a Diesel), we would return an Err value with the appropriate TrainError.
         let fuel_status = self.check_fuel()?;
         
         Ok(format!("Departure Status: {}, Fuel Status: {:?}", engine_status, fuel_status))
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
            //println!("Train Car {}: Engine Personality - {}, Fuel Level - {:?}", car.id, describe_personality(&car.engine), car.fuel_level);
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

        //let ok_car_ids: String = ok_engine_line.iter().map(|&car| car.id.to_string()).collect::<Vec<String>>().join(", ");

        //Ok(format!("{}:::::::::Train {} has {} cars ready for departure! Car(s): [{}]",ok_start, self.id, ok_engine_line.len(), ok_car_ids))
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





struct Railyard {
    trains: Vec<Train>,
    cars: HashMap<u32, TrainCar>,
    //cargo: Vec<Cargo>,
}

impl Railyard {
    
    fn new() -> Self {
        Railyard {
            trains: Vec::new(),
            cars: HashMap::new(),
            //cargo: Vec::new(),
        }
    }
    


    fn house(&mut self, train: Train) {
        self.trains.push(train);
    }

    fn add_car(&mut self, car: TrainCar) {
        self.cars.insert(car.id, car);
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
            self.add_car(car);
            println!(
                "Decoupled Car {}, from position {} in Train {} and added it to the railyard.",
                id,
                pos,
                train.id
            );
        } else {
            println!("Car {} is not attached to Train {}.", id, train.id);
        }
    }



    /*
    fn couple(&mut self, train: &mut Train, car: TrainCar) {
        // Logic to couple a car to a train
        // For example, we could add the car to the train's list of cars and remove it from the railyard's list of cars
        if let Some(pos) = self.cars.iter().position(|c| c.id == car.id) {
            let removed_car = self.cars.remove(pos);
            train.cars.push(removed_car);
            println!("Coupled Car {} to Train {} and removed it from the railyard.", car.id, train.id);
        } else {
            println!("Car {} is not available in the railyard.", car.id);
        }
    }
*/



/*
    fn decouple(&mut self, train:  &mut Train, car: &mut TrainCar) {
        // Logic to decouple a car from a train
        // For example, we could remove the car from the train's list of cars and add it to the railyard's list of cars
        if let Some(pos) = train.cars.iter().position(|c| c.id == car.id) {
            let removed_car = train.cars.remove(pos);
            self.cars.insert(removed_car.id, removed_car);
            println!("Decoupled Car {} from Train {} and added it to the railyard.", car.id, train.id);
        } else {
            println!("Car {} is not attached to Train {}.", car.id, train.id);
        }
    }
*/



/*
    fn add_cargo(&mut self, cargo: Cargo) {
        self.cargo.push(cargo);
    }
*/



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
        train.rehabilitate();
        train.refuel();
        
        let mut ok_cars: Vec<TrainCar> = Vec::new();

        for car in train.cars.drain(..) {
            match car.prepare_for_departure() {
                Ok(msg) => {
                    println!("Train Car {} is ready for departure: {}", car.id, msg);
                    ok_cars.push(car);
                }
                Err(e) => {
                    println!("Train Car {} cannot depart: {:?}. Pushing to Railyard.", car.id, e);
                    self.cars.insert(car.id, car);
                }
            }
        }
        train.cars = ok_cars;
        train
    }
        

        
}   







fn main() {


    //let mut the_line: Vec<TrainCar> = Vec::new();
    /*let mut the_line: Train = Train {
        id: 1,
        cars: Vec::<TrainCar>::new(),
    };*/

    let mut yard: Railyard = Railyard {
        trains: Vec::new(),
        cars: HashMap::new(),
        //cargo: Vec::new(),
    };

    let cargo1 = Cargo { item: String::from("bananas"), manifest_weight: 1000, actual_weight: 1000, contraband: None };
    let cargo2 = Cargo { item: String::from("crates of oranges"), manifest_weight: 1000, actual_weight: 1005, contraband: Some(String::from("Stylish TUMI Briefcase")) };
    let cargo3 = Cargo { item: String::from("Redacted Documents"), manifest_weight: 2000, actual_weight: 9001, contraband: Some(String::from("The Service Weapon")) };
    let cargo4 = Cargo { item: String::from("Various Crafting Ingredients"), manifest_weight: 1500, actual_weight: 1500, contraband: None };
    let cargo5 = Cargo { item: String::from("Scrap Metal"), manifest_weight: 10000, actual_weight: 10075, contraband: Some(String::from("Excessively Heavy Fire Extinguisher")) };
    let cargo6 = Cargo { item: String::from("pallets of electronics"), manifest_weight: 3000, actual_weight: 3000, contraband: None };


    let carriage = TrainCar { id:1, cargo: Some(cargo2), passenger: Some(String::from("Lemon:"))};
    let dining_car = TrainCar { id:2, cargo: Some(cargo1), passenger: Some(String::from("Ladybug"))};
    let boxcar1 = TrainCar { id:3, cargo: Some(cargo5), passenger: Some(String::from("Blazkowicz")),};
    let boxcar2 = TrainCar { id:4, cargo: Some(cargo6), passenger: Some(String::from("Tangerine")),};
    let boxcar3 = TrainCar { id:5, cargo: Some(cargo3), passenger: Some(String::from("Faden")),};
    let caboose = TrainCar { id:6, cargo: Some(cargo4), passenger: Some(String::from("Artyom"))};

    let mut the_line = Train {
        id: 1,
        engine: EngineType::Diesel,
        fuel_level: FuelLevel::Low,
        //cars: vec![carriage, dining_car, boxcar, caboose],
        cars: Vec::new(),
    };

    yard.add_car(carriage);
    yard.add_car(dining_car);
    yard.add_car(boxcar1);
    yard.add_car(boxcar2);
    yard.add_car(boxcar3);
    yard.add_car(caboose);


    // transfer cars from the yard into the_line by identifier; the local vars have
    // already been moved into the yard, so we can't use them again.
    yard.couple_by_id(&mut the_line, 1);
    yard.couple_by_id(&mut the_line, 2);
    yard.couple_by_id(&mut the_line, 3);
    yard.couple_by_id(&mut the_line, 4);
    yard.couple_by_id(&mut the_line, 5);
    yard.couple_by_id(&mut the_line, 6);


    //the_line = yard.service_train(the_line);

    the_line.dispatch().map(|ok_cars| {
        let ok_car_ids: String = ok_cars.iter()
            .map(|car| car.id.to_string())
            .collect::<Vec<String>>()
            .join(", ");
        println!("Train {} has {} cars ready for departure! Car(s): [{}]", the_line.id, ok_cars.len(), ok_car_ids);
    }).unwrap_or_else(|e| println!("Error dispatching the train: {:?}", e));


    println!("The total cargo weight on train {} is {} kg.", the_line.id, the_line.calculate_cargo_weight());


    //yard.decouple_by_id(&mut the_line, 1);

    the_line.dispatch().map(|ok_cars| {
        let ok_car_ids: String = ok_cars.iter()
            .map(|car| car.id.to_string())
            .collect::<Vec<String>>()
            .join(", ");
        println!("Train {} has {} cars ready for departure! Car(s): [{}]", the_line.id, ok_cars.len(), ok_car_ids);
    }).unwrap_or_else(|e| println!("Error dispatching the train: {:?}", e));

    println!("The total cargo weight on train {} is {} kg.", the_line.id, the_line.calculate_cargo_weight());

    the_line = yard.service_train(the_line);

    the_line.dispatch().map(|ok_cars| {
        let ok_car_ids: String = ok_cars.iter()
            .map(|car| car.id.to_string())
            .collect::<Vec<String>>()
            .join(", ");
        println!("Train {} has {} cars ready for departure! Car(s): [{}]", the_line.id, ok_cars.len(), ok_car_ids);
    }).unwrap_or_else(|e| println!("Error dispatching the train: {:?}", e));

    println!("The total cargo weight on train {} is {} kg.", the_line.id, the_line.calculate_cargo_weight());
}



fn describe_personality(engine: &EngineType) -> String{
    match engine {
        EngineType::Thomas => String::from("Thomas is a friendly and helpful engine, always ready to lend a hand and make friends. Thomas is the most popular."),
        EngineType::Percy => String::from("Percy is a brave and intuitive little engine that doesn't always think things through, but always does his best. Percy is the most adventurous."),
        EngineType::Gordon => String::from("Gordon is proud and doesn't like to admit when he's wrong, but he cares about his friends. Gordon is the strongest."),
        EngineType::Diesel => String::from("Diesel is a troublemaker."),
    }
}
