/*struct TrainCar {
    id: u32,
    contents: String,
}
*/

enum EngineType {
    Diesel,
    Thomas,
    Percy,
}

fn main() {


let beckett = EngineType::Diesel;
let description = describe_personality(beckett);
println!("Beckett's personality: {}", description);

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