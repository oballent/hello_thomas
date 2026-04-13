mod models;
mod facilities;
mod network;

use crate::models::{Cargo, Engine, EngineType, FreightOrder, Location, Mission, MissionReport, StationCommand, TrainCar};
use crate::facilities::Station;
use crate::network::{RailwayNetwork, GlobalLedger};

use core::net;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::collections::{HashMap};

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";

// This program demonstrates the physics of Rust and Networking in the context of a train yard on the Island of Sodor. It has evolved from a simple simulation of trains moving between stations to a more complex system that includes concurrent producer threads submitting missions to a central network, which then dispatches trains across the network to fulfill those missions. The program uses Rust's ownership model, concurrency primitives, and error handling to create a robust and realistic simulation of a railway network.
// I AM The Fat Controller, or, at least, I would be, if this were built around a centralized God Object: All aboard for a Rustacean adventure on the Island of Sodor! Choo choo!

fn main() {



    // 1. Create the raw data
    let ledger = GlobalLedger::new();
    
    // 2. Put it in the Mutex Vault (The Talking Stick)
    let locked_ledger = Mutex::new(ledger);
    
    // 3. Put the Vault in an Arc so multiple threads can find it
    let shared_ledger = Arc::new(locked_ledger);

    let mut network = RailwayNetwork::new();

    let mut temporary_switchboard: HashMap<u32, Sender<StationCommand>> = HashMap::new();

    // Create the Tidmouth radio BEFORE the Tidmouth thread exists
    let tidmouth_id = 0;
    let (tidmouth_tx, tidmouth_rx) = mpsc::channel();
    temporary_switchboard.insert(tidmouth_id, tidmouth_tx.clone());
    let tidmouth_location = Location { x: 100.0, y: 100.0 };
    let tidmouth_name = "Tidmouth".to_string();
    let tidmouth_md = facilities::StationMetadata { id: tidmouth_id, name: tidmouth_name.clone(), location: tidmouth_location };
    network.register_station(tidmouth_id, tidmouth_md.location);

    let brendam_docks_id = 1;
    let (brendam_docks_tx, brendam_docks_rx) = mpsc::channel();
    temporary_switchboard.insert(brendam_docks_id, brendam_docks_tx.clone());
    let brendam_docks_location = Location { x: 100.0, y: 350.0 };
    let brendam_docks_name = "Brendam Docks".to_string();
    let brendam_docks_md = facilities::StationMetadata { id: brendam_docks_id, name: brendam_docks_name.clone(), location: brendam_docks_location };
    network.register_station(brendam_docks_id, brendam_docks_md.location);

    let knapford_id = 2;
    let (knapford_tx, knapford_rx) = mpsc::channel();
    temporary_switchboard.insert(knapford_id, knapford_tx.clone());
    let knapford_location = Location { x: 300.0, y: 100.0 };
    let knapford_name = "Knapford".to_string();
    let knapford_md = facilities::StationMetadata { id: knapford_id, name: knapford_name.clone(), location: knapford_location };
    network.register_station(knapford_id, knapford_md.location);

    let welsworth_id = 3;
    let (welsworth_tx, welsworth_rx) = mpsc::channel();
    temporary_switchboard.insert(welsworth_id, welsworth_tx.clone());
    let welsworth_location = Location { x: 300.0, y: 350.0 };
    let welsworth_name = "Welsworth".to_string();
    let welsworth_md = facilities::StationMetadata { id: welsworth_id, name: welsworth_name.clone(), location: welsworth_location };
    network.register_station(welsworth_id, welsworth_md.location);

    let maron_id = 4;
    let (maron_tx, maron_rx) = mpsc::channel();
    temporary_switchboard.insert(maron_id, maron_tx.clone());
    let maron_location = Location { x: 500.0, y: 100.0 };
    let maron_name = "Maron".to_string();
    let maron_md = facilities::StationMetadata { id: maron_id, name: maron_name.clone(), location: maron_location };
    network.register_station(maron_id, maron_md.location);

    let vicarstown_id = 5;
    let (vicarstown_tx, vicarstown_rx) = mpsc::channel();
    temporary_switchboard.insert(vicarstown_id, vicarstown_tx.clone());
    let vicarstown_location = Location { x: 500.0, y: 350.0 };
    let vicarstown_name = "Vicarstown".to_string();
    let vicarstown_md = facilities::StationMetadata { id: vicarstown_id, name: vicarstown_name.clone(), location: vicarstown_location };
    network.register_station(vicarstown_id, vicarstown_md.location);

    let peel_godred_id = 6;
    let (peel_godred_tx, peel_godred_rx) = mpsc::channel();
    temporary_switchboard.insert(peel_godred_id, peel_godred_tx.clone());
    let peel_godred_location = Location { x: 700.0, y: 100.0 };
    let peel_godred_name = "Peel Godred".to_string();
    let peel_godred_md = facilities::StationMetadata { id: peel_godred_id, name: peel_godred_name.clone(), location: peel_godred_location };
    network.register_station(peel_godred_id, peel_godred_md.location);


    network.add_track(tidmouth_md.id, knapford_md.id); //we only need to pass references one way because the track is bidirectional. Once the track is laid, both stations can access it through the network's internal data structures.
    network.add_track(tidmouth_md.id, peel_godred_md.id);
    network.add_track(knapford_md.id, welsworth_md.id);
    network.add_track(knapford_md.id, maron_md.id);
    network.add_track(welsworth_md.id, brendam_docks_md.id);
    network.add_track(welsworth_md.id, maron_md.id);
    network.add_track(maron_md.id, vicarstown_md.id);

    
    // let mut tidmouth_neighborhood: Vec<u32> = Vec::new();
    // tidmouth_neighborhood.push(knapford_id);
    // tidmouth_neighborhood.push(peel_godred_id);
    // let mut knapford_neighborhood: Vec<u32> = Vec::new();
    // knapford_neighborhood.push(tidmouth_id);
    // knapford_neighborhood.push(welsworth_id);
    // knapford_neighborhood.push(maron_id);
    // let mut welsworth_neighborhood: Vec<u32> = Vec::new();
    // welsworth_neighborhood.push(knapford_id);
    // welsworth_neighborhood.push(brendam_docks_id);
    // welsworth_neighborhood.push(maron_id);
    // let mut maron_neighborhood: Vec<u32> = Vec::new();
    // maron_neighborhood.push(knapford_id);
    // maron_neighborhood.push(welsworth_id);
    // maron_neighborhood.push(vicarstown_id);
    // let mut vicarstown_neighborhood: Vec<u32> = Vec::new();
    // vicarstown_neighborhood.push(maron_id);
    // let mut peel_godred_neighborhood: Vec<u32> = Vec::new();
    // peel_godred_neighborhood.push(tidmouth_id);
    // let mut brendam_docks_neighborhood: Vec<u32> = Vec::new();
    // brendam_docks_neighborhood.push(welsworth_id);





    // //Copilot was here, helping with syntax as usual. Thanks, GitHub! P.S. I know this looks a bit repetitive, but it's just setting up the initial neighbor relationships for each station based on the tracks we laid. Each station needs to know who its neighbors are so it can communicate with them and send trains in the right direction when fulfilling missions. This is a crucial part of setting up the network before we start spawning station threads and dispatching missions.
    // let tidmouth_neighbors = tidmouth_neighborhood.into_iter().map(|id| (id, temporary_switchboard.get(&id).expect("Neighbor station channel must exist").clone())).collect();
    // let knapford_neighbors = knapford_neighborhood.into_iter().map(|id| (id, temporary_switchboard.get(&id).expect("Neighbor station channel must exist").clone())).collect();
    // let welsworth_neighbors = welsworth_neighborhood.into_iter().map(|id| (id, temporary_switchboard.get(&id).expect("Neighbor station channel must exist").clone())).collect();
    // let maron_neighbors = maron_neighborhood.into_iter().map(|id| (id, temporary_switchboard.get(&id).expect("Neighbor station channel must exist").clone())).collect();
    // let vicarstown_neighbors = vicarstown_neighborhood.into_iter().map(|id| (id, temporary_switchboard.get(&id).expect("Neighbor station channel must exist").clone())).collect();
    // let peel_godred_neighbors = peel_godred_neighborhood.into_iter().map(|id| (id, temporary_switchboard.get(&id).expect("Neighbor station channel must exist").clone())).collect();
    // let brendam_docks_neighbors = brendam_docks_neighborhood.into_iter().map(|id| (id, temporary_switchboard.get(&id).expect("Neighbor station channel must exist").clone())).collect();






    // ... right after all your network.add_track() calls ...

    // A helper to automatically build the neighbors HashMap by reading the tracks
    let build_neighbors = |station_id: u32, net: &RailwayNetwork, switch: &HashMap<u32, Sender<StationCommand>>| -> HashMap<u32, Sender<StationCommand>> {
        let mut local_neighbors = HashMap::new();
        
        // Look at the mathematical tracks we just laid
        if let Some(destinations) = net.get_track(&station_id) {
            for (neighbor_id, _distance) in destinations {
                // Grab the radio for this specific neighbor
                let tx = switch.get(neighbor_id).expect("Missing tx!").clone();
                local_neighbors.insert(*neighbor_id, tx);
            }
        }
        local_neighbors
    };

    // Now, instantly build the HashMaps for the Stations:
    let tidmouth_neighbors = build_neighbors(tidmouth_id, &network, &temporary_switchboard);
    let knapford_neighbors = build_neighbors(knapford_id, &network, &temporary_switchboard);
    let welsworth_neighbors = build_neighbors(welsworth_id, &network, &temporary_switchboard);
    let maron_neighbors = build_neighbors(maron_id, &network, &temporary_switchboard);
    let vicarstown_neighbors = build_neighbors(vicarstown_id, &network, &temporary_switchboard);
    let peel_godred_neighbors = build_neighbors(peel_godred_id, &network, &temporary_switchboard);
    let brendam_docks_neighbors = build_neighbors(brendam_docks_id, &network, &temporary_switchboard);








    let shared_network = Arc::new(network);


    // 1. Instantiate the Stations locally
    let tidmouth = Station::new(tidmouth_md.id, &tidmouth_md.name, tidmouth_neighbors, tidmouth_tx, Arc::clone(&shared_network), tidmouth_rx);
    let knapford = Station::new(knapford_md.id, &knapford_md.name, knapford_neighbors, knapford_tx, Arc::clone(&shared_network), knapford_rx);
    let welsworth = Station::new(welsworth_md.id, &welsworth_md.name, welsworth_neighbors, welsworth_tx, Arc::clone(&shared_network), welsworth_rx);
    let maron = Station::new(maron_md.id, &maron_md.name, maron_neighbors, maron_tx, Arc::clone(&shared_network), maron_rx);
    let vicarstown = Station::new(vicarstown_md.id, &vicarstown_md.name, vicarstown_neighbors, vicarstown_tx, Arc::clone(&shared_network), vicarstown_rx);
    let peel_godred = Station::new(peel_godred_md.id, &peel_godred_md.name, peel_godred_neighbors, peel_godred_tx, Arc::clone(&shared_network), peel_godred_rx);
    let brendam_docks = Station::new(brendam_docks_md.id, &brendam_docks_md.name, brendam_docks_neighbors, brendam_docks_tx, Arc::clone(&shared_network), brendam_docks_rx);

    let cargo1 = Cargo { item: String::from("bananas"), actual_weight: 1000, contraband: None };
    let cargo2 = Cargo { item: String::from("crates of oranges"), actual_weight: 1005, contraband: Some(String::from("Stylish TUMI Briefcase")) };
    let cargo3 = Cargo { item: String::from("Redacted Documents"), actual_weight: 11001, contraband: Some(String::from("The Service Weapon")) };
    let cargo4 = Cargo { item: String::from("Various Crafting Ingredients"), actual_weight: 1500, contraband: None };
    let cargo5 = Cargo { item: String::from("Scrap Metal"), actual_weight: 10075, contraband: Some(String::from("Excessively Heavy Fire Extinguisher")) };
    let cargo6 = Cargo { item: String::from("pallets of electronics"), actual_weight: 3000, contraband: None };
    let cargo7 = Cargo { item: String::from("Declassified Documents"), actual_weight: 11001, contraband: Some(String::from("The Truth")) };

    let carriage = TrainCar { id:1, cargo: Some(cargo2), passenger: Some(String::from("Lemon:"))};
    let dining_car = TrainCar { id:2, cargo: Some(cargo1), passenger: Some(String::from("Ladybug"))};
    let boxcar1 = TrainCar { id:3, cargo: Some(cargo5), passenger: Some(String::from("Blazkowicz")),};
    let boxcar2 = TrainCar { id:4, cargo: Some(cargo6), passenger: Some(String::from("Tangerine")),};
    let boxcar3 = TrainCar { id:5, cargo: Some(cargo3), passenger: Some(String::from("Faden")),}; 
    let boxcar4 = TrainCar { id:5, cargo: Some(cargo7), passenger: Some(String::from("Mathison")),};
    let caboose = TrainCar { id:6, cargo: Some(cargo4), passenger: Some(String::from("Artyom"))};

    let tidmouth_incoming_cars = vec![carriage, dining_car, boxcar1, boxcar2, boxcar3, boxcar4, caboose];


    let engine1 = Engine { id: 1, engine_type: EngineType::Thomas, current_fuel: 1000.0 };
    let engine2 = Engine { id: 2, engine_type: EngineType::Thomas, current_fuel: 3000.0 };
    let engine3 = Engine { id: 3, engine_type: EngineType::Percy, current_fuel: 750.0 };
    let engine4 = Engine { id: 4, engine_type: EngineType::Diesel, current_fuel: 500.0 };
    let engine5 = Engine { id: 5, engine_type: EngineType::Gordon, current_fuel: 5000.0 };

    let tidmouth_incoming_engines = vec![engine1, engine2, engine3, engine4, engine5];
    // We're going to add engines and cars to the station before we add the station to the network. This is a bit like setting up the station's inventory and resources before it starts receiving missions and dispatching trains. Since we're still in the main thread and haven't moved the station into the network yet, we can freely mutate it without worrying about ownership conflicts with the network. Once we add the station to the network, it will be owned by the network and we won't be able to directly access it from the main thread anymore, but that's okay because the station will be able to receive commands and send updates through its own channels.
    let (tx_reply, rx_reply) = mpsc::channel();
    // TESTING: We can send commands to the station before it's fully integrated into the network, as long as we have its channel set up. This allows us to simulate the process of the station receiving resources and preparing for operations even before it starts handling missions from the network. It's a bit like stocking the station with supplies and engines before it starts running trains, which is a realistic part of how a station would operate in the real world.
    // match tidmouth_tx.clone().send(StationCommand::IntakeCar { cars: tidmouth_incoming_cars, reply_to: tx_reply.clone() }) {
    match temporary_switchboard.get(&0).expect("Tidmouth channel must exist").clone().send(StationCommand::IntakeCar { cars: tidmouth_incoming_cars, reply_to: tx_reply.clone() }) {
        Ok(_) => println!("Car successfully intaken by Tidmouth!"),
        Err(e) => println!("Failed to intake car: {:?}", e),
    }
    //We will block and wait for Tidmouth to confirm that it has received the car before we send the next one. This simulates a more realistic process where the station needs to acknowledge receipt of each car before accepting the next one, and it also allows us to see the flow of messages between the main thread and the station more clearly in the console output.
    match rx_reply.recv() {
        Ok(result) => match result {
            Ok(_) => println!("Tidmouth confirmed receipt of the car."),
            Err(e) => println!("Tidmouth reported an error while intaking the car: {:?}", e),
        },
        Err(e) => println!("Failed to receive reply from Tidmouth: {:?}", e),
    };



    // TESTING. We can do the same thing with engines, sending them one at a time and waiting for confirmation from Tidmouth before sending the next one. This allows us to simulate the process of the station receiving and processing each engine individually, which is more realistic and also helps us see the message flow more clearly in the console output. It's a bit like how a station would need to inspect and prepare each engine before it can be added to the roundhouse and used for operations.
    tidmouth_incoming_engines.into_iter().for_each(|engine| {
        match temporary_switchboard.get(&0).expect("Tidmouth channel must exist").clone().send(StationCommand::IntakeEngine { engine, reply_to: tx_reply.clone() }) {
            Ok(_) => println!("Engine successfully intaken by Tidmouth!"),
            Err(e) => println!("Failed to intake engine: {:?}", e),
        }
        match rx_reply.recv() {
            Ok(result) => match result {
                Ok(_) => println!("Tidmouth confirmed receipt of the engine."),
                Err(e) => println!("Tidmouth reported an error while intaking the engine: {:?}", e),
            },
            Err(e) => println!("Failed to receive reply from Tidmouth: {:?}", e),
        }
    });


    
    // 2. Build the tracks using immutable references to the local variables!
    // network gets mutated, but tidmouth and brendam_docks are merely read. No conflict.






    //Before creating freight orders, we make cargo
    let foam1 = Cargo { item: "foam".to_string(), actual_weight: 1, contraband: None};
    let foam2 = Cargo { item: "foam".to_string(), actual_weight: 1, contraband: None};
    let foam3 = Cargo { item: "foam".to_string(), actual_weight: 1, contraband: None};

    // //now to use the foam to create some freight orders
    // let freight_order1 = FreightOrder { cargo: foam1, origin: "Tidmouth".to_string(), destination: "Brendam Docks".to_string() };
    // let freight_order2 = FreightOrder { cargo: foam2, origin: "Tidmouth".to_string(), destination: "Brendam Docks".to_string() };
    // let freight_order3 = FreightOrder { cargo: foam3, origin: "Tidmouth".to_string(), destination: "Brendam Docks".to_string() };
    // //Testing that this whole .lock() thing really works! Choo!
    // {
    //     let mut ledger_access = shared_ledger.lock().unwrap();
    //     ledger_access.pending_cargo.push(freight_order1);
    //     ledger_access.pending_cargo.push(freight_order2);
    //     ledger_access.pending_cargo.push(freight_order3);
    // }




    
    // let mission1 = Mission { 
    //     id: 1, 
    //     request_id: 1001,
    //     origin: String::from("Tidmouth"), 
    //     destination: String::from("Brendam Docks"), 
    //     required_cars: vec![2, 4], 
    //     reply_channel: Some(tx_reply1) 
    // };



    println!("{YELLOW}System Online. Spawning independent customer threads...{RESET}");




// for id in shared_network.tracks.keys() {
//     println!("Station {} has tracks to: {:?}", id, shared_network.tracks.get(id).unwrap().iter().map(|(dest, _)| dest).collect::<Vec<&u32>>());

// }





    // 3. Spawn Producer 1
    let network_clone_1 = Arc::clone(&shared_network);
    let switchboard_clone_1 = temporary_switchboard.clone();
    let ledger_for_p1 = Arc::clone(&shared_ledger);
    let producer_1_handle = thread::spawn(move || {
        println!("Producer 1: Submitting Mission 1 to the Network...");
        let switchboard = switchboard_clone_1; // The producer can use the switchboard to send commands to stations
        // 1. Wait in line for the Talking Stick
        // let mut ledger_access = ledger_for_p1.lock().unwrap();

        // // 3. The lock automatically drops when `ledger_access` goes out of scope!

        // loop {
        //     println!("test_loop");
        //     // 1. Create a temporary variable to hold our assignment (if we get one)
        //     let my_assignment: Option<FreightOrder> = {
                
        //         // --- LOCK ACQUIRED ---
        //         let mut ledger_access = ledger_for_p1.lock().unwrap();
                
        //         // 2. You now have exclusive, mutable access to the GlobalLedger!

        //         println!("There are currently {} items waiting to be shipped.", ledger_access.pending_cargo.len());
        //         // We act like a Hungry Hippo: just pop the last item off the list.
        //         // If the list is empty, pop() returns None.
        //         ledger_access.pending_cargo.pop() 
                
        //     }; // --- LOCK DROPPED AUTOMATICALLY HERE! ---
        //     // 2. Now we are outside the lock. The ledger is free for other threads.
        //     if let Some(freight_order) = my_assignment {
        //         println!("Producer claimed {}kg of {}. Building mission...", freight_order.weight, freight_order.description);

        //         // Build your Mission for this single piece of cargo
        //         // tx.send(StationCommand::AssembleMission { ... })
        //     } else {
        //         // No cargo available. Sleep for a second before checking again.
        //         println!("test_sleep");
        //         thread::sleep(std::time::Duration::from_secs(1));
        //     }
        // }
        
        
        
        
        
        
        
        
        
        
        
        
        
        println!("Producer 1: Submitting Mission 1 to the Network...");
        //The Producer threads will create the Mission payloads and send them to the Network. The Network will then process these missions by dispatching trains across the network to fulfill them. After processing each mission, the Network will send a report back to the respective producer thread through the reply channel included in the mission payload, allowing the producers to track the status of their missions and print out the results.
        let (tx_reply1, rx_reply1) = mpsc::channel();
        let mission1 = Mission { 
            id: 1, 
            request_id: 1001,
            origin: 0,
            destination: 1, 
            required_cars: vec![2, 4], 
            reply_channel: Some(tx_reply1) 
        };
        // Network creates the Conductor in the background!
        let (transit_tx, transit_rx) = mpsc::channel();
        //network_clone_1.dispatch_train_across_network(mission1); 
        switchboard.get(&0).expect("Tidmouth channel must exist").clone().send(StationCommand::AssembleMission { mission: mission1.clone(), reply_to: transit_tx }).expect("Failed to send AssembleMission command to Tidmouth");
        
        // Wait for the final report from the Conductor
        if let Ok(report) = rx_reply1.recv() {
            println!("Producer 1 received report: {:?}", report);
        }
    });

    // 4. Spawn Producer 2
    let network_clone_2 = Arc::clone(&shared_network);
    let switchwboard_clone_2 = temporary_switchboard.clone();

    let ledger_for_p2 = Arc::clone(&shared_ledger);
    let producer_2_handle = thread::spawn(move || {
        // println!("Producer 2: Submitting Mission 2 to the Network...");
        // let switchboard = switchwboard_clone_2; // The producer can use the switchboard to send commands to stations
        
        // // 1. Wait in line for the Talking Stick
        // let mut ledger_access = ledger_for_p2.lock().unwrap();


        // // 3. The lock automatically drops when `ledger_access` goes out of scope!

        // loop {
        //     println!("test_loop_2");
        //     // 1. Create a temporary variable to hold our assignment (if we get one)
        //     let my_assignment: Option<FreightOrder> = {
                
        //         // 1. Wait in line for the Talking Stick
        //         // --- LOCK ACQUIRED ---
        //         let mut ledger_access = ledger_for_p2.lock().unwrap();

        //         // 2. You now have exclusive, mutable access to the GlobalLedger!
        //         println!("There are currently {} items waiting to be shipped.", ledger_access.pending_cargo.len());
                
        //         // We act like a Hungry Hippo: just pop the last item off the list.
        //         // If the list is empty, pop() returns None.
        //         ledger_access.pending_cargo.pop() 
                
        //     }; // --- LOCK DROPPED AUTOMATICALLY HERE! ---

        //     // 2. Now we are outside the lock. The ledger is free for other threads.
        //     if let Some(freight_order) = my_assignment {
        //         println!("Producer claimed {}kg of {}. Building mission...", freight_order.weight, freight_order.description);

        //         // Build your Mission for this single piece of cargo
        //         // tx.send(StationCommand::AssembleMission { ... })
        //     } else {
        //         // No cargo available. Sleep for a second before checking again.
        //         thread::sleep(std::time::Duration::from_secs(1));
        //     }
        // }
        
        
        
        
        
        
        
        
        
        
        
        //Same thing for Producer 2, but with a different mission!
        let (tx_reply2, rx_reply2) = mpsc::channel();
        let mission2 = Mission{
            id:2, 
            request_id: 2002,
            origin: 0,
            destination: 1,
            required_cars: vec![6],
            reply_channel: Some(tx_reply2)
        };

        println!("Producer 2: Submitting Mission 2 to the Network...");

        let (transit_tx, transit_rx) = mpsc::channel();
        switchwboard_clone_2.get(&0).expect("Tidmouth channel must exist").clone().send(StationCommand::AssembleMission { mission: mission2.clone(), reply_to: transit_tx }).expect("Failed to send AssembleMission command to Tidmouth");
        //network_clone_2.dispatch_train_across_network(mission2); 
        
        if let Ok(report) = rx_reply2.recv() {
            println!("Producer 2 received report: {:?}", report);
        }
    });

    // 5. The "smart wait" for the producers to finish. We don't want to just sleep the main thread for an arbitrary amount of time; we want to actually wait for the producer threads to complete their work before we proceed with printing the final station status and shortest route. By calling join() on each producer thread handle, we ensure that the main thread will block until each producer thread has finished executing, which means we'll have received all the mission reports and printed them out before we move on to the next steps in the main thread.
    //thread::sleep(std::time::Duration::from_secs(6)); // This is just to ensure that the producers have time to send their missions and receive their reports before we print the final status. In a more complex simulation, we would want to implement a more robust synchronization mechanism to ensure that all threads have completed their work before we proceed, but for this simple example, a short sleep is sufficient to allow the message passing to complete before we print the final results.
    println!("{YELLOW}Waiting for producer threads to complete...{RESET}");
        producer_1_handle.join().unwrap();
        producer_2_handle.join().unwrap();
    println!("{BOLD}{GREEN}Simulation Complete.{RESET}");




}


