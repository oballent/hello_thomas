use std::clone;

// This program demonstrates the concept of mutable references in Rust using a simple example of train engines and their personalities.
struct TrainCar {
    id: u32,
    engine: EngineType,
    passenger: Option<String>,
    fuel_level: FuelLevel,
}


//#[derive(Clone, Copy)] // This allows us to easily create copies of EngineType values, which is useful for passing them around without losing ownership.
enum EngineType {
    Diesel,
    Thomas,
    Percy,
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
}

impl TrainCar {
    fn rehabilitate(&mut self) {
        println!("Rehabilitating the train car's engine...");
        self.engine = EngineType::Thomas;
    }

    fn refuel(&mut self) {
        println!("Refueling the train car...");
        self.fuel_level = FuelLevel::Full;
    }

    fn check_passenger(&self) {
        match &self.passenger {
            Some(name) => println!("{} is aboard!", name),
            None => println!("Ain't nobody on this train car!"),
        }
    }

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

    fn prepare_for_departure(&self) -> Result<String, TrainError> {
        //How come we no longer reference self.start_engine() with &self.start_engine()? Is it because we are already borrowing self in the method signature, so we can call self.start_engine() directly without needing to borrow it again? Yes, that's correct! Since the method signature already borrows self as an immutable reference (&self), we can call other methods on self directly without needing to borrow it again. The Rust compiler understands that we are working with a borrowed reference to self and allows us to call methods on it without needing to explicitly borrow it again. So in this case, we can simply call self.start_engine() without needing to use &self.start_engine(). The compiler will handle the borrowing for us and ensure that we are using the borrowed reference correctly.
         let engine_status = self.start_engine()?;
         // Where does OK(String::from("The train is ready for departure!")) come from? Is it just a way to return a successful result from the function, indicating that the train is ready for departure? Yes, that's correct! The Ok(String::from("The train is ready for departure!")) is a way to return a successful result from the prepare_for_departure() function. It indicates that the engine started successfully and the train is ready for departure. The Ok variant of the Result type is used to represent a successful outcome, while the Err variant is used to represent an error. In this case, if the engine starts successfully, we return an Ok value with a message indicating that the train is ready for departure. If there was an error starting the engine (like if it's a Diesel), we would return an Err value with the appropriate TrainError.
         let fuel_status = self.check_fuel()?;
         
         Ok(format!("Departure Status: {}, Fuel Status: {:?}", engine_status, fuel_status))
    }
}

fn main() {

/*
let mut car_7 = TrainCar{
    id: 7,
    engine: EngineType::Diesel,
};

println!("Car 7's engine personality: {}", describe_personality(&car_7.engine));

car_7.rehabilitate();

println!("Car 7's engine personality after rehabilitation: {}", describe_personality(&car_7.engine));*/


/*
//let beckett = &mut EngineType::Diesel;
let mut beckett: TrainCar = TrainCar{
    id: 7,
    engine: EngineType::Diesel,
    passenger: Some(String::from("Lemon")),
};
*/

/*
let mut diesel_himself = TrainCar{
    id: 8,
    engine: EngineType::Diesel,
    passenger: None,
};
*/

//What does it really mean to have keep ownership in main? It means that we can create a mutable variable that holds the engine type, and we can pass a mutable reference to it when we want to rehabilitate its personality. This way, we can modify the engine's personality without losing ownership of the variable in main. But we can also pass a reference to the variable when we want to describe its personality, without needing to modify it. This allows us to keep ownership of the variable in main while still being able to interact with it in different ways.
//println!("Beckett's personality: {}", describe_personality(&beckett.engine));

//beckett.rehabilitate();
//println!("Beckett's personality after rehabilitation: {}", describe_personality(&beckett.engine));


//beckett.check_passenger();
//diesel_himself.check_passenger();

let mut the_line: Vec<TrainCar> = Vec::new();

the_line.push(TrainCar { id: 1, engine: EngineType::Thomas, passenger: Some(String::from("Lemon")), fuel_level: FuelLevel::Low });
the_line.push(TrainCar { id: 2, engine: EngineType::Diesel, passenger: None, fuel_level: FuelLevel::Low });
the_line.push(TrainCar { id: 3, engine: EngineType::Percy, passenger: Some(String::from("Tangerine")), fuel_level: FuelLevel::Full });

for car in &the_line {
    //println!("Train Car {}: Engine Personality - {}, Fuel Level - {:?}", car.id, describe_personality(&car.engine), car.fuel_level);
    match car.prepare_for_departure() {
        Ok(msg) => println!("Train Car {}: {}", car.id, msg),
        Err(e) => {
            println!("Train Car {}: Error preparing for departure: {:?}", car.id, e);
            println!("--- Dispatcher: Skipping car {} and moving to next... ---", car.id);
        }
    }
}

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
        EngineType::Diesel => String::from("Diesel is a troublemaker, always causing mischief and chaos on the tracks."),
        EngineType::Thomas => String::from("Thomas is a friendly and helpful engine, always ready to lend a hand and make friends."),
        EngineType::Percy => String::from("Percy is a brave and intuitive little engine that doesn't always think things through, but always tries his best."),
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