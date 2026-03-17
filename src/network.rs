use crate::models::{Mission, TrainError};
use crate::facilities::Station;
use std::collections::HashMap;

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";


pub struct RailwayNetwork {
    // Maps (Origin, Destination) -> Distance in km
    tracks: HashMap<(String, String), u32>,
    // The network also needs to hold the Stations themselves so it can route trains between them
    stations: HashMap<String, Station>,
    missions: HashMap<u32, Mission>, // <-- The Source of Truth for all missions on the network
}

impl RailwayNetwork {
    pub fn new() -> Self {
        RailwayNetwork {
            tracks: HashMap::new(),
            stations: HashMap::new(),
            missions: HashMap::new(),
        }
    }
    
    pub fn add_station(&mut self, station: Station) {
        // station.name is a String. We can clone it to use as the key, 
        // and move the actual station into the value.
        self.stations.insert(station.name.clone(), station);
    }

    pub fn add_tracks(&mut self, origin: &Station, destination: &Station, distance: u32) {
        // Clone the names to own them inside the tuple key
        let route = (origin.name.clone(), destination.name.clone());
        self.tracks.insert(route, distance);
        
        // Sodor is not a one-way street. You likely want the reverse route too!
        let return_route = (destination.name.clone(), origin.name.clone());
        self.tracks.insert(return_route, distance);
    }

    pub fn add_mission(&mut self, mission: Mission) {
        println!("{YELLOW}Network Ledger: Registered Mission {}.{RESET}", mission.id);
        self.missions.insert(mission.id, mission);
    }

    pub fn get_distance(&self, origin: &Station, destination: &Station) -> Option<u32> {
        self.tracks.get(&(origin.name.clone(), destination.name.clone())).copied()
    }

    /// Allows the global environment (main) to temporarily borrow a station to mutate it.
    pub fn get_mut_station(&mut self, name: &str) -> Option<&mut Station> {
        self.stations.get_mut(name)
    }

    pub fn get_mission(&self, mission_id: &u32) -> Option<&Mission> {
        self.missions.get(mission_id)
    }

    // Note: passing primitive integers by reference (&u32) is unidiomatic. Just pass u32!
    pub fn dispatch_train_across_network(&mut self, mission_id: u32) {
        
        // 1. Direct Field Access. 
        // This immutably locks ONLY self.missions. self.stations is still free!
        let mission = match self.missions.get(&mission_id) {
            Some(m) => m,
            None => {
                println!("{RED}Network Error: Mission {} does not exist in the ledger.{RESET}", mission_id);
                return; 
            }
        };

        // We clone the names here because we will need to re-insert the stations later
        // and we cannot move data out of our borrowed mission reference.
        let origin_name = mission.origin.clone();
        let dest_name = mission.destination.clone();

        // 2. Amputate the Origin (mutably borrowing ONLY self.stations)
        let mut origin = match self.stations.remove(&origin_name) {
            Some(s) => s,
            None => {
                println!("{RED}Network Error: Origin station '{}' not found.{RESET}", origin_name);
                return;
            }
        };

        // 3. Amputate the Destination
        let mut destination = match self.stations.remove(&dest_name) {
            Some(s) => s,
            None => {
                println!("{RED}Network Error: Destination station '{}' not found.{RESET}", dest_name);
                self.stations.insert(origin_name, origin); // Put the origin back before aborting!
                return;
            }
        };

        // 4. Calculate Distance
        let distance = self.get_distance(&origin, &destination).unwrap_or(0);
        println!("Distance from {} to {} is {} km", origin.name, destination.name, distance);

        // 5. The Execution (We pass our single, original `mission` reference!)
        if let Ok(mut train) = origin.assemble_and_dispatch(mission, distance) {
            if let Ok(_) = train.dispatch() {
                println!("{GREEN}Mission {} completed successfully! Train {} has arrived at {}.{RESET}", mission.id, train.id, destination.name);
                destination.receive_train(train);
            } else {
                println!("{RED}TRAIN NOT RECEIVED: Mission {} from {} to {} failed during traversal.{RESET}", mission.id, origin.name, destination.name);
            }
        } else {
            println!("{RED}Mission {} from {} to {} failed to assemble.{RESET}", mission.id, origin.name, destination.name);
        }

        // 6. Return ownership back to the network
        self.stations.insert(origin_name, origin); 
        self.stations.insert(dest_name, destination);
    }

    // pub fn dispatch_train_across_network(&mut self, mission_id: &u32) {

    //     let origin_name: String;
    //     let destination_name: String;
    //     if let Some(m) = self.get_mission(mission_id) {
    //         origin_name = m.origin.clone();
    //         destination_name = m.destination.clone();
    //     } else {
    //         println!("{RED}Network Error: Mission {} does not exist in the ledger.{RESET}", mission_id);
    //         return; // Abort before touching any stations
    //     }
    //     let mut origin = self.stations.remove(&origin_name).expect("Origin station not found");
    //     let mut destination = self.stations.remove(&destination_name).expect("Destination station not found");

    //     // let distance: u32 = self.tracks.get(&(origin.name.clone(), destination.name.clone()))
    //     //     .copied()
    //     //     .unwrap_or(0);

    //     // println!("Dispatching mission {} from {} to {} ({} km)", mission_id, origin.name, destination.name, distance);

    //     let distance = self.get_distance(&origin, &destination).unwrap_or(0);
    //     println!("Distance from {} to {} is {} km", origin.name, destination.name, distance);
    //     if let Some(mission) = self.get_mission(mission_id) {
    //         if let Ok(mut train) = origin.assemble_and_dispatch(mission, distance) {
    //             if let Ok(_) = train.dispatch() {
    //                 println!("{GREEN}Mission {} completed successfully! Train {} has arrived at {}.{RESET}", mission.id, train.id, destination.name);
    //                 destination.receive_train(train); // The train arrives at the destination station, which triggers the disassembly process.
    //             } else {
    //                 println!("{RED}TRAIN NOT RECEIVED:Mission {} from {} to {} failed to dispatch during traversal.{RESET}", mission.id, origin.name, destination.name);
    //             }
    //         } else {
    //             println!("{RED}Mission {} from {} to {} failed to dispatch.{RESET}", mission.id, origin.name, destination.name);
    //         }
    //     }
    //     self.stations.insert(origin.name.clone(), origin); // Return ownership back to the network
    //     self.stations.insert(destination.name.clone(), destination);
    // }

}
