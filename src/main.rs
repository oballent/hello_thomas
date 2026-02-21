struct TrainCar {
    id: u32,
    contents: String,
}

fn main() {
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
}

// This function takes a MUTABLE reference to a TrainCar
fn fix_engine(car: &mut TrainCar) {
    // We replace the 'Diesel' clay with 'Thomas' clay
    car.contents = String::from("Thomas");
}