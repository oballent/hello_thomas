/*struct TrainCar {
    id: u32,
    contents: String,
}
*/

use std::f32::consts::E;
#[derive(Copy, Clone)] // This is the "Magic Stamp"
enum EngineType {
    Diesel,
    Thomas,
    Percy,
}

fn main() {


//let beckett = &mut EngineType::Diesel;
let mut beckett: EngineType = EngineType::Diesel;

println!("Beckett's personality: {}", describe_personality(beckett));

rehabilitate(&mut beckett);

println!("Beckett's personality after rehabilitation: {}", describe_personality(beckett));


/*
    // 1. The car itself must be 'mut' so we can change it later
    let mut car = TrainCar {
        id: 7,
        contents: String::from("Diesel"),
    };

    println!("Current Car: {} contains {}", car.id, car.contents);

    // 2. We check the car (Reading)
    if car.contents == "Diesel" {
        println!("Warning: Troublemaker detected. Initiating attitude adjustment...");
        
        // 3. We fix the car (Writing/Mutating)
        // We pass it as a mutable reference using '&mut'
        fix_engine(&mut car);
    }

    println!("After adjustment: Car {} now contains {}", car.id, car.contents);
    println!("{} says, 'Don't nick.'", car.contents);
    */
}


// This function takes a MUTABLE reference to a TrainCar
/*
fn fix_engine(car: &mut TrainCar) {
    // We replace the 'Diesel' clay with 'Thomas' clay
    car.contents = String::from("Thomas");
}
*/

fn describe_personality(engine: EngineType) -> String{
    match engine {
        EngineType::Diesel => String::from("Diesel is a troublemaker, always causing mischief and chaos on the tracks."),
        EngineType::Thomas => String::from("Thomas is a friendly and helpful engine, always ready to lend a hand and make friends."),
        EngineType::Percy => String::from("Percy is a brave and intuitive little engine that doesn't always think things through, but always tries his best."),
    }
}

fn rehabilitate(engine: &mut EngineType) {
    println!("Rehabilitating the engine's personality...");
    // This function would contain logic to rehabilitate the engine's personality
    // For example, if it's a Diesel, we could change it to a Thomas
    *engine = EngineType::Thomas;
}