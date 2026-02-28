use std::clone;

// This program demonstrates the concept of mutable references in Rust using a simple example of train engines and their personalities.
struct TrainCar {
    id: u32,
    cargo: Option<String>,
    passenger: Option<String>,
    contraband: Option<String>,
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
    ContrabandOnBoard,
    NoCargoOrPassengers,
}

impl TrainCar {
    fn check_passenger(&self) {
        match &self.passenger {
            Some(name) => println!("{} is aboard!", name),
            None => println!("Ain't nobody on this train car!"),
        }
    }

    fn check_cargo(&self) {
        match &self.cargo {
            Some(item) => println!("Cargo on board: {}", item),
            None => println!("Ain't no cargo on this #@$! train car!"),
        }
    }

    fn check_freight(&self) -> Result<String, TrainError> {
        match (&self.cargo, &self.passenger) {
            (None, None) => Err(TrainError::NoCargoOrPassengers),
            (Some(cargo), None) => Ok(format!("Cargo on board: {}", cargo)),
            (None, Some(passenger)) => Ok(format!("Passenger aboard: {}", passenger)),
            (Some(cargo), Some(passenger)) => Ok(format!("Cargo on board: {}. Passenger aboard: {}", cargo, passenger)),
        }
        
        /*
        if self.cargo.is_none() && self.passenger.is_none() {
            Err(TrainError::NoCargoOrPassengers)
        } else {
            Ok(String::from("Passengers or cargo aboard this car!"))
        }
        */
    }

    fn check_contraband(&self) -> Result<String, TrainError> {
        match &self.contraband {
            Some(item) => Err(TrainError::ContrabandOnBoard),
            None => Ok(String::from("No contraband aboard this car!")),
        }

        /*
        if self.contraband.is_some() {
            Err(TrainError::ContrabandOnBoard)
        } else {
            Ok(String::from("No contraband aboard this car!"))
        }
        */
    }

    
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

    fn dispatch(&self) -> Result<String, TrainError> {
        /*
        match self.prepare_for_departure() {
            Ok(msg) => println!("Train {} is ready for departure: {}", self.id, msg),
            Err(e) => {
                println!("Train {} cannot depart: {:?}", self.id, e);
                return Err(e);
            }
        }
        */

        let ok: String = String::from(format!("Train {} is ready for departure!: {}", self.id, self.prepare_for_departure()?));

        println!("Train {} has {} cars to prepare for departure!", self.id, self.cars.len());
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

        let ok_car_ids: String = ok_engine_line.iter().map(|&car| car.id.to_string()).collect::<Vec<String>>().join(", ");

        Ok(format!("{}:::::::::Train {} has {} cars ready for departure! Car(s): [{}]",ok, self.id, ok_engine_line.len(), ok_car_ids))

            
    }
}


fn main() {


//let mut the_line: Vec<TrainCar> = Vec::new();
/*let mut the_line: Train = Train {
    id: 1,
    cars: Vec::<TrainCar>::new(),
};*/

let carriage = TrainCar { id:1, cargo: None, passenger: Some(String::from("Lemon:")), contraband: Some(String::from("briefcase full of money"))};
let dining_car = TrainCar { id:2, cargo: None, passenger: Some(String::from("Ladybug")), contraband: None};
let boxcar = TrainCar { id:3, cargo: None, passenger: None, contraband: None};
let caboose = TrainCar { id:4, cargo: Some(String::from("bananas")), passenger: Some(String::from("Tangerine")), contraband: None};

let mut the_line = Train {
    id: 1,
    engine: EngineType::Diesel,
    fuel_level: FuelLevel::Low,
    cars: vec![carriage, dining_car, boxcar, caboose],
};

/*
let mut the_line = Train {
    id: 1,
    cars: Vec::new(),
};


the_line.cars.push(thomas);
the_line.cars.push(diesel);
the_line.cars.push(percy);
*/

/*
for car in &the_line.cars {
    //println!("Train Car {}: Engine Personality - {}, Fuel Level - {:?}", car.id, describe_personality(&car.engine), car.fuel_level);
    match car.prepare_for_departure() {
        Ok(msg) => println!("Train Car {}: {}", car.id, msg),
        Err(e) => {
            println!("Train Car {}: Error preparing for departure: {:?}", car.id, e);
            println!("--- Dispatcher: Skipping car {} and moving to next... ---", car.id);
        }
    }
}
*/
match the_line.dispatch() {
    Ok(msg) => println!("{}", msg),
    Err(e) => println!("Error dispatching the train: {:?}", e),
}
the_line.rehabilitate();
match the_line.dispatch() {
    Ok(msg) => println!("{}", msg),
    Err(e) => println!("Error dispatching the train: {:?}", e),
}


the_line.refuel();
match the_line.dispatch() {
    Ok(msg) => println!("{}", msg),
    Err(e) => println!("Error dispatching the train: {:?}", e),
}



/* 
//let test_line = vec![&thomas_car, &diesel_car, &percy_car];
let ok_engine_line: Vec<&TrainCar> = the_line.cars.iter()// // 1. Start the conveyor belt
        //why do we put & before car in the filter closure? Is it because we are iterating over references to TrainCar objects, so we need to dereference them to access their methods and properties? Yes, that's correct! When we use the iter() method on the vector of TrainCar objects, it returns an iterator that yields references to the TrainCar objects. Therefore, in the filter closure, we receive a reference to a TrainCar (let's call it car), and we need to dereference it (using &car) to access its methods and properties. This is because the methods like prepare_for_departure() are defined on the TrainCar struct, and we need to dereference the reference to call those methods on the actual TrainCar object. So by using &car in the filter closure, we are able to access the methods and properties of the TrainCar objects correctly.
        .filter(|&car| car.prepare_for_departure().is_ok()) // 2. "Filter" out the Diesels
        .collect(); // 3. Put the survivors into a new Box (Vec)

    println!("The OK_Engine line has {} useful engines.", ok_engine_line.len());

*/






/*

let thomas = TrainCar { id: 1, engine: EngineType::Thomas, passenger: Some(String::from("Lemon")), fuel_level: FuelLevel::Low };
let diesel = TrainCar { id: 2, engine: EngineType::Diesel, passenger: None, fuel_level: FuelLevel::Low };
let percy = TrainCar { id: 3, engine: EngineType::Percy, passenger: Some(String::from("Tangerine")), fuel_level: FuelLevel::Full };

let the_line = vec![thomas, diesel, percy]; // Shorthand to create a Vec

    // THE ITERATOR PIPELINE:
    let ok_engine_line: Vec<&TrainCar> = the_line.iter() // 1. Start the conveyor belt
        .filter(|car| car.prepare_for_departure().is_ok()) // 2. "Filter" out the Diesels
        .collect(); // 3. Put the survivors into a new Box (Vec)

    println!("The OK_Engine line has {} useful engines.", ok_engine_line.len());

*/







/*

//let mut car: TrainCar = TrainCar { id: 9, engine: EngineType::Diesel, passenger: None, fuel_level: FuelLevel::Low };

match car.start_engine() {
    Ok(message) => println!("{}", message),
    Err(error) => println!("Error starting the engine: {:?}", error),
}

match car.check_fuel() {
    Ok(_) => println!("Fuel level is sufficient for departure."),
    Err(error) => println!("Error checking fuel level: {:?}", error),
}

match car.prepare_for_departure() {
    Ok(status) => println!("{}", status),
    Err(error) => println!("Error preparing for departure: {:?}", error),
}

//car.prepare_for_departure();
//car.prepare_for_departure().map(|status| println!("{}", status)).unwrap_or_else(|error| println!("Error preparing for departure: {:?}", error));

car.rehabilitate();

match car.start_engine() {
    Ok(message) => println!("{}", message),
    Err(error) => println!("Error starting the engine: {:?}", error),
}

match car.check_fuel() {
    Ok(_) => println!("Fuel level is sufficient for departure."),
    Err(error) => println!("Error checking fuel level: {:?}", error),
}

match car.prepare_for_departure() {
    Ok(status) => println!("{}", status),
    Err(error) => println!("Error preparing for departure: {:?}", error),
}

car.refuel();



match car.start_engine() {
    Ok(message) => println!("{}", message),
    Err(error) => println!("Error starting the engine: {:?}", error),
}

match car.check_fuel() {
    Ok(_) => println!("Fuel level is sufficient for departure."),
    Err(error) => println!("Error checking fuel level: {:?}", error),
}

match car.prepare_for_departure() {
    Ok(status) => println!("{}", status),
    Err(error) => println!("Error preparing for departure: {:?}", error),
}

*/

//car.prepare_for_departure();
//car.prepare_for_departure().map(|status| println!("{}", status)).unwrap_or_else(|error| println!("Error preparing for departure: {:?}", error));

}



fn describe_personality(engine: &EngineType) -> String{
    match engine {
        EngineType::Thomas => String::from("Thomas is a friendly and helpful engine, always ready to lend a hand and make friends. Thomas is the best."),
        EngineType::Percy => String::from("Percy is a brave and intuitive little engine that doesn't always think things through, but always does his best."),
        EngineType::Gordon => String::from("Gordon is proud and doesn't like to admit when he's wrong, but he cares deeply about his friends, and he's the strongest."),
        EngineType::Diesel => String::from("Diesel is a troublemaker, always causing mischief and chaos on the tracks."),
    }
}

/*
fn rehabilitate(engine: &mut EngineType) {
    println!("Rehabilitating the engine's personality...");
    // This function would contain logic to rehabilitate the engine's personality
    // For example, if it's a Diesel, we could change it to a Thomas
    *engine = EngineType::Thomas;
} 
*/