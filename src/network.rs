use crate::models::{Mission, MissionReport, TrainError};
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
        
        // Sodor is not a one-way street!
        let return_route = (destination.name.clone(), origin.name.clone());
        self.tracks.insert(return_route, distance);
    }

    pub fn add_mission(&mut self, mission: Mission) {
        println!("{YELLOW}Network Ledger: Registering Mission {}.{RESET}", mission.id);
        self.missions.insert(mission.id, mission);
    }

    pub fn get_station(&self, name: &str) -> Option<&Station> {
        self.stations.get(name)
    }
    
    pub fn get_distance(&self, origin: &String, destination: &String) -> Option<u32> {
        self.tracks.get(&(origin.clone(), destination.clone())).copied()
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
        // The mission is The Oracle that reveals the metadata we need to perform the dispatch, but it does not have the power to mutate anything itself. It's just a reference to the immutable ledger.
        let mission = match self.missions.get(&mission_id) {
            Some(m) => m,
            None => {
                println!("{RED}Network Error: Mission {} does not exist in the ledger.{RESET}", mission_id);
                return; 
            }
        };


        //1. Do the Stations exist? (Check the Nodes)
        if !self.stations.contains_key(&mission.origin) {
            println!("Error: Origin missing."); return;
        }
        if !self.stations.contains_key(&mission.destination) {
            println!("Error: Destination missing."); return;
        }

        // 2. Does the Route exist? (Check the Edges)
        let distance = match self.get_distance(&mission.origin, &mission.destination) {
            Some(d) => d,
            None => {
                println!("Error: No track laid between {} and {}.", mission.origin, mission.destination); 
                return;
            }
        };

        // We clone the names here because we will need to re-insert the stations later
        // and we cannot move data out of our borrowed mission reference.
        let origin_name = mission.origin.clone();
        let dest_name = mission.destination.clone();

        // 3. Isolate the Origin (mutably borrowing ONLY self.stations)
        let mut origin = self.stations.remove(&origin_name).expect("Origin station not found"); // We can safely unwrap here because we already checked for existence above. This is the moment we take ownership of the station to mutate it.

        // 4. Isolate the Destination
        let mut destination = self.stations.remove(&dest_name).expect("Destination station not found"); // We can safely unwrap here because we already checked for existence above. This is the moment we take ownership of the station to mutate it.

        // 5. The Execution (We pass our single, original `mission` reference!)
        let report = if let Ok(mut train) = origin.assemble_and_dispatch(mission, distance) {
            if let Ok(_) = train.dispatch() {
                //println!("{GREEN}Mission {} completed successfully! Train {} has arrived at {}.{RESET}", mission.id, train.id, destination.name);
                destination.receive_train(train);
                MissionReport::Success(format!("Train arrived safely at {}", dest_name))
            } else {
                //println!("{RED}TRAIN NOT RECEIVED: Mission {} from {} to {} failed during traversal.{RESET}", mission.id, origin.name, destination.name);
                MissionReport::Failure(format!("Train failed to arrive at {}", dest_name))
            }
        } else {
            //println!("{RED}Mission {} from {} to {} failed to assemble.{RESET}", mission.id, &origin.name, &destination.name);
            MissionReport::Failure(format!("Mission {} from {} to {} failed to assemble.", mission.id, &origin.name, &destination.name))
        };

        if let Some(reply_tx) = &mission.reply_channel {
            let _ = reply_tx.send(report);
        }

        // 6. Return ownership back to the network
        self.stations.insert(origin_name.clone(), origin); 
        self.stations.insert(dest_name.clone(), destination);


    }


}