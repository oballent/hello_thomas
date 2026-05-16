use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;
use tracing::warn;

// 1. The wrapper to hold a station and its cumulative distance in the queue
#[derive(Clone, PartialEq)]
struct RouteState {
    cost: f64,
    station: u32,
}

// 2. We promise the compiler we can check for absolute equality
impl Eq for RouteState {}

// 3. THE MAGIC FLIP: We teach Rust how to compare RouteStates.
// By flipping `other` and `self`, we trick the Max-Heap into acting like a Min-Heap!
impl Ord for RouteState {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for RouteState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}



use crate::models::Location;

#[derive(Debug, Clone, Default)]
pub struct TelemetrySnapshot {
    pub cargo_registered: u32,
    pub cargo_delivered: u32,
    pub cargo_failed: u32,
    pub cargo_terminal: u32, // Includes both delivered and failed cargo, as both are considered "terminal" states for a cargo item.
    pub cargo_expired_in_warehouse: u32,
    pub cargo_went_to_purgatory: u32,
    pub cargo_expired_in_purgatory: u32,
    pub trains_derailed: u32,
}

impl TelemetrySnapshot {
    pub fn active_cargo(&self) -> u32 {
        self.cargo_registered.saturating_sub(self.cargo_terminal)
    }

    pub fn is_drained(&self) -> bool {
        self.cargo_registered > 0 && self.cargo_terminal >= self.cargo_registered
    }
}

#[derive(Debug, Clone)]
pub enum TelemetryEvent {
    CargoRegistered { cargo_ids: Vec<u32> },
    CargoDelivered { cargo_ids: Vec<u32> },
    CargoExpiredInWarehouse { cargo_ids: Vec<u32> },
    CargoExpiredInPurgatory { cargo_ids: Vec<u32> },
    CargoSentToPurgatory { cargo_ids: Vec<u32> },
    TrainDerailed,
}

enum TelemetryCommand {
    Record(TelemetryEvent),
    GetSnapshot {
        reply_to: Sender<TelemetrySnapshot>,
    },
    Shutdown,
}

pub struct TelemetryLedger {
    tx: Sender<TelemetryCommand>,
}

#[derive(Clone)]
pub struct TelemetryClient {
    tx: Sender<TelemetryCommand>,
}

impl TelemetryLedger {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel::<TelemetryCommand>();

        thread::spawn(move || {
            let mut snapshot = TelemetrySnapshot::default();
            let mut registered_cargo_ids = HashSet::new(); // These HashSets are like Gatekeepers. They make sure we only count each cargo ID once for each category, preventing double-counting if the same cargo ID appears in multiple events.
            let mut delivered_cargo_ids = HashSet::new();
            let mut failed_cargo_ids = HashSet::new();
            let mut terminal_cargo_ids = HashSet::new();
            let mut purgatory_cargo_ids = HashSet::new();

            while let Ok(command) = rx.recv() {
                match command {
                    TelemetryCommand::Record(event) => match event {
                        TelemetryEvent::CargoRegistered { cargo_ids } => {
                            for cargo_id in cargo_ids {
                                if registered_cargo_ids.insert(cargo_id) {
                                    snapshot.cargo_registered = snapshot.cargo_registered.saturating_add(1);
                                }
                            }
                        }
                        TelemetryEvent::CargoDelivered { cargo_ids } => {
                            for cargo_id in cargo_ids {
                                if delivered_cargo_ids.insert(cargo_id) {
                                    snapshot.cargo_delivered = snapshot.cargo_delivered.saturating_add(1);
                                }
                                if terminal_cargo_ids.insert(cargo_id) {
                                    snapshot.cargo_terminal = snapshot.cargo_terminal.saturating_add(1);
                                }
                            }
                        }
                        TelemetryEvent::CargoExpiredInWarehouse { cargo_ids } => {
                            snapshot.cargo_expired_in_warehouse = snapshot
                                .cargo_expired_in_warehouse
                                .saturating_add(cargo_ids.len() as u32);

                            for cargo_id in cargo_ids {
                                if failed_cargo_ids.insert(cargo_id) {
                                    snapshot.cargo_failed = snapshot.cargo_failed.saturating_add(1);
                                }
                                if terminal_cargo_ids.insert(cargo_id) {
                                    snapshot.cargo_terminal = snapshot.cargo_terminal.saturating_add(1);
                                }
                            }
                        }
                        TelemetryEvent::CargoExpiredInPurgatory { cargo_ids } => {
                            snapshot.cargo_expired_in_purgatory = snapshot
                                .cargo_expired_in_purgatory
                                .saturating_add(cargo_ids.len() as u32);

                            for cargo_id in cargo_ids {
                                if failed_cargo_ids.insert(cargo_id) {
                                    snapshot.cargo_failed = snapshot.cargo_failed.saturating_add(1);
                                }
                                if terminal_cargo_ids.insert(cargo_id) {
                                    snapshot.cargo_terminal = snapshot.cargo_terminal.saturating_add(1);
                                }
                            }
                        }
                        TelemetryEvent::CargoSentToPurgatory { cargo_ids } => {
                            for cargo_id in cargo_ids {
                                if purgatory_cargo_ids.insert(cargo_id) {
                                    snapshot.cargo_went_to_purgatory =
                                        snapshot.cargo_went_to_purgatory.saturating_add(1);
                                }
                            }
                        }
                        TelemetryEvent::TrainDerailed => {
                            snapshot.trains_derailed = snapshot.trains_derailed.saturating_add(1);
                        }
                    },
                    TelemetryCommand::GetSnapshot { reply_to } => {
                        let _ = reply_to.send(snapshot.clone());
                    }
                    TelemetryCommand::Shutdown => break,
                }
            }
        });

        Self { tx }
    }

    pub fn client(&self) -> TelemetryClient {
        TelemetryClient {
            tx: self.tx.clone(),
        }
    }

    pub fn snapshot(&self) -> Option<TelemetrySnapshot> {
        let (reply_tx, reply_rx) = mpsc::channel();
        if self
            .tx
            .send(TelemetryCommand::GetSnapshot { reply_to: reply_tx })
            .is_err()
        {
            return None;
        }

        reply_rx.recv_timeout(Duration::from_millis(250)).ok()
    }

    pub fn shutdown(&self) {
        let _ = self.tx.send(TelemetryCommand::Shutdown);
    }

}

impl TelemetryClient {
    pub fn record(&self, event: TelemetryEvent) {
        let _ = self.tx.send(TelemetryCommand::Record(event));
    }
}




// Using type aliases to make the code highly readable without sacrificing performance
pub type StationId = u32;
pub type Distance = f64;


pub struct RailwayNetwork {
    // Maps (Origin, Destination) -> Distance in km
    tracks: HashMap<StationId, Vec<(StationId, Distance)>>,
    // We keep this purely for UI/Debugging translation, NOT for logic.
    //pub station_names: HashMap<StationId, String>,
    //missions: HashMap<u32, Mission>, // <-- The Source of Truth for all missions on the network
    station_locations: HashMap<u32, Location>
}

impl RailwayNetwork {
    pub fn new() -> Self {
        RailwayNetwork {
            tracks: HashMap::new(),
            //stations: HashMap::new(),
            //station_names: HashMap::new(),
            //missions: HashMap::new(),
            station_locations: HashMap::new(),
        }
    }
    


    pub fn register_station(&mut self, id: u32, location: Location) {
        self.station_locations.insert(id, location);
    }


    pub fn add_track(&mut self, a: u32, b: u32) {
        // 1. Look up the locations from our internal directory
        let loc_a = self.station_locations.get(&a).expect("Station A not found!");
        let loc_b = self.station_locations.get(&b).expect("Station B not found!");

        // 2. Do the math internally (No manual work for the user!)
        let distance = loc_a.distance_to(loc_b);

        // 3. Insert both directions automatically
        // We use `entry().or_insert_with()` to either get the existing vector of tracks for that station or create a new one if it doesn't exist. Then we push the new track onto that vector. This way, we maintain a complete list of all tracks connected to each station.
        
        
        // wanna clean this up a bit, Copilot? We're checking the map twice and doing some redundant work. Maybe we can just do one check and then insert if it doesn't exist?
        
        if let Some(neighbors) = self.tracks.get(&a) {
            if neighbors.iter().any(|(dest, _)| *dest == b) {
                //println!("{YELLOW}Network: Track already exists between {} and {}. Skipping.{RESET}", a, b);
                warn!("Network: Track already exists between {} and {}. Skipping.", a, b);
                return;
            }
        }
        
        //info!("Network: Laying track between {} and {} ({:.2}km)", a, b, distance);
        self.tracks.entry(a).or_insert_with(Vec::new).push((b, distance));
        self.tracks.entry(b).or_insert_with(Vec::new).push((a, distance));
        //info!("Network: Track laid between {} and {} ({:.2}km)", a, b, distance);
        
    }

    pub fn get_distance(&self, origin: StationId, destination: StationId) -> Option<Distance> {
        // We create a temporary tuple of StationId objects to match the HashMap key signature.
        //for v in self.tracks.get(&origin) {
        if let Some(v) = self.tracks.get(&origin) {
            for (dest, dist) in v {
                if *dest == destination {//dereference the reference to compare the actual value
                    return Some(*dist);//dereference the reference to return the actual value
                }
            }
        };
        None
    }

    // pub fn get_mission(&self, mission_id: &u32) -> Option<&Mission> {
    //     self.missions.get(mission_id)
    // }



    // Returns an Option containing a tuple: (Total Distance, Vector of Station Names in order)
    pub fn find_shortest_path(&self, origin: StationId, destination: StationId) -> Option<(Distance, Vec<StationId>)> {
        
        // 1. The Scoreboard: Tracks the shortest known cumulative distance to each station
        let mut distances: HashMap<StationId, Distance> = HashMap::new();
        
        // 2. The Breadcrumbs: Remembers the previous station so we can retrace our steps at the end
        let mut came_from: HashMap<StationId, StationId> = HashMap::new();
        
        // 3. The Queue: Our Min-Heap that always gives us the closest cumulative station
        let mut priority_queue = BinaryHeap::new();

        // Initialize: Set all known stations to Infinity
        for station in self.station_locations.keys() {
            distances.insert(*station, f64::INFINITY);
        }

        // START THE WAVE: The distance from the origin to itself is 0.0
        distances.insert(origin, 0.0);
        priority_queue.push(RouteState { cost: 0.0, station: origin });

        // --- THE ALGORITHM LOOP GOES HERE ---
        // while let Some(RouteState { cost, station }) = priority_queue.pop() {
        //     ...
        // }

        // ... (Previous setup: HashMap initialization, pushing origin to queue)

        while let Some(RouteState { cost, station }) = priority_queue.pop() {
            
            // 1. THE STALE TICKET CHECK
            // If we pull a ticket that is worse than our current scoreboard, throw it away.
            let known_best = *distances.get(&station).unwrap_or(&f64::INFINITY);
            if cost > known_best {
                continue;// Copilot was here. They said this is the key optimization that keeps Dijkstra's algorithm efficient. Without this check, we would process every single path to every station, even if we already found a better one. Thanks, Copilot!
            }

            // 2. THE DESTINATION CHECK
            // If the station we just popped is our destination, we are done! The shortest path is locked.
            if station == destination {
                // Time to follow the breadcrumbs backwards!
                let mut path = Vec::new();
                let mut current = destination;
                
                while let Some(previous) = came_from.get(&current) {
                    path.push(current);
                    current = previous.clone();
                }
                path.push(origin);
                path.reverse(); // Flip it so it goes Origin -> Destination
                
                return Some((cost, path));
            }

            // 3. THE SCOUTING PHASE
            // We are at a valid station. Let's look at all the tracks connected to it.
            
            if let Some(v) = self.tracks.get(&station) {
                for (track_dest, track_distance) in v {
                    // Calculate the cumulative distance to this neighbor
                    let next_cost = cost + track_distance;
                    let neighbor_best = *distances.get(track_dest).unwrap_or(&f64::INFINITY);

                    // 4. THE DISCOVERY
                    // If this new path is strictly better than what the neighbor currently has...
                    if next_cost < neighbor_best {
                        // ...Update the scoreboard!
                        distances.insert(*track_dest, next_cost);
                        // ...Leave a breadcrumb pointing back to how we got here!
                        came_from.insert(*track_dest, station);
                        // ...Print a new ticket and throw it in the queue!
                        priority_queue.push(RouteState { cost: next_cost, station: *track_dest });
                    }
                }
            };
        }

        // If the queue empties and we never hit the `if station == destination` block, 
        // it means there is physically no track connecting them.
        //None

        None // Temporary return
    }

    pub fn get_tracks(&self, station_id: &u32) -> Option<&Vec<(StationId, Distance)>> {
        self.tracks.get(station_id)
    }

}