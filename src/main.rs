// dawfu: Da Watch Face Uploader - Face Uploader for MO YOUNG / DA FIT Smart Watches using Bluetooth LE (via btleplug)
// Copyright 2022 David Atkinson <david@47k@d47.co> (remove the first @)
// MIT License

use std::io;
use std::io::Write;
use std::error::Error;
use std::time::Duration;
use tokio::time;
use tokio_stream::StreamExt;
use uuid::Uuid;
use btleplug::api::{
    Central, 
    Manager as _, 
    Peripheral, 
    ScanFilter, 
    bleuuid::*,
    CharPropFlags,
    WriteType,
    Characteristic
};
use btleplug::platform::Manager;
use std::env;
use std::convert::TryInto;

const WAIT_TIME: u64 = 10;


//
// UUID constants
//
const SU_BATTERY: Uuid = uuid_from_u16(0x180f);         // Battery Service
const CU_BATTERY: Uuid = uuid_from_u16(0x2a19);         // Battery Level

const SU_DEVINFO: Uuid = uuid_from_u16(0x180a);         // Device Information Service
const CU_SERIALNUM: Uuid = uuid_from_u16(0x2a25);       // Serial Number String
const CU_SOFTREV: Uuid = uuid_from_u16(0x2a28);         // Software Revision String
const CU_MANUFACTURER: Uuid = uuid_from_u16(0x2a29);    // Manufacturer Name String

const _SU_D0FF: Uuid = uuid::uuid!("0000d0ff-3c17-d293-8e48-14fe2e4da212");
const _SU_FEE7: Uuid = uuid_from_u16(0xfee7);

const SU_FEEA: Uuid = uuid_from_u16(0xfeea);
const CU_SEND: Uuid = uuid_from_u16(0xfee2);
const CU_SENDFILE: Uuid = uuid_from_u16(0xfee6);
const _CU_NOTIFYX: Uuid = uuid_from_u16(0xfee1);
const CU_NOTIFY: Uuid = uuid_from_u16(0xfee3);


//
// IsNotEmpty implementation
//
// If clippy is going to suggest that string != "" is LESS clear than !string.is_empty(), 
// then we'll take it a step further: It would look better as string.is_not_empty()
//
pub trait IsNotEmpty {
    fn is_not_empty(&self) -> bool; 
}

impl IsNotEmpty for String {
    fn is_not_empty(&self) -> bool {
        !self.is_empty()
    }
}

impl IsNotEmpty for Vec<u8> {
    fn is_not_empty(&self) -> bool {
        !self.is_empty()
    }
}


//
// Application modes
//
#[derive(PartialEq)]
enum Mode { Help, Info, Upload }


//
// Main function
//
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    println!("da_watch_face_uploader: Face Uploader for MO YOUNG / DA FIT Smart Watches");
    let mut device_name: String = "".to_string();
    let mut device_address: String = "".to_string();
    let mut filename: String = "".to_string();
    let mut verbosity: u32 = 0;    
    let mut selected_adapter: Option<usize> = None;

    let mut _slot: u32 = 13;      // From 1 to 13, counting the watch faces on the watch and in DaFit app. 13 is the Watch Gallery, 6 is the user watch face.
                                 // When setting watch face, we use 0x0D (13). When specifying slot, we use 0x74 (decimal 116) which is 103d + 13d.
                                 // When setting watch face 6, we use file 0x6E (decimal 110), which is 104d + 6d.

    // process command-line arguments
    let args: Vec<String> = env::args().collect();
    let mode: Mode = if args.len() < 2 {
        Mode::Help
    } else {
        match &args[1][..] {
            "info" => Mode::Info,
            "upload" => Mode::Upload,
            _ => Mode::Help,
        }
    };

    for arg in args.iter().skip(2) {
        if arg.contains('=') {
            let idx = arg.find('=').unwrap();
            let lhs = (arg[0..idx]).to_string();
            let rhs = (arg[idx+1..]).to_string();
            match &lhs[..] {
                "name"      => device_name      = rhs,
                "address"   => device_address   = rhs,
                "verbosity" => verbosity        = rhs.parse::<u32>().unwrap(),
                "adapter"   => selected_adapter = Some(rhs.parse::<usize>().unwrap()),
                _           => filename         = arg.clone(),
            };
        } else {
            filename = arg.clone();
        }
    }

    if mode == Mode::Help {
        println!("usage: dawfu mode [options] [filename]");
        println!("mode:        info                        Show device information.");
        println!("             upload                      Upload a binary watch file.");
        println!("             help                        Show this help information.");
        println!("options:     name=MyWatch                Limit to devices with matching name.");
        println!("             address=01:23:45:67:89:ab   Limit to devices with matching address.");
        println!("             verbosity=1                 Set debug message verbosity.");
        println!("             adapter=1                   Select which bluetooth adapter to use.");
        println!("filename:                                File to upload.");
        println!();
        return Ok(());
    }

    let mut filedata: Vec::<u8> = Vec::new();
    if filename.is_not_empty() && mode == Mode::Upload { // open the file, read the whole lot to memory
        filedata = std::fs::read(filename)?;
        // calculate quick checksum.
        // I don't actually know what they use for checksum!
        //let mut sum: i32 = 0;
        //for i in 0..filedata.len() {
        //    sum += (filedata[i] as i8) as i32;
        //}
        //println!("File checksum: {:08x}", sum);
    }

    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters found");
    }


    if adapter_list.len() > 1 {
        println!("More than one bluetooth adapter found.\n");
        for (n,adapter) in adapter_list.iter().enumerate() {
            println!("Adapter {}: {}\n", n, adapter.adapter_info().await?);
        }
        if selected_adapter == None {
            println!("Defaulting to the first adapter. Select adapter with adapter=N argument.\n");
            selected_adapter = Some(0);
        }
    }

    let adapter = &adapter_list[selected_adapter.unwrap()];
    println!("Starting Bluetooth (BLE) scan using adapter {}...", adapter.adapter_info().await?);
    adapter
        .start_scan(ScanFilter::default())
        .await
        .expect("Can't scan for connected devices with Bluetooth (BLE) adapter!");
    time::sleep(Duration::from_secs(WAIT_TIME)).await;

    let peripherals = adapter.peripherals().await?;
    if peripherals.is_empty() {
        eprintln!("No BLE peripheral devices found.");
        return Ok(());
    }

    // All peripheral devices in range
    let mut device_found = false;
    for peripheral in peripherals.iter() {
        if device_found {
            break;
        }
        let properties = peripheral.properties().await?;
        let is_connected = peripheral.is_connected().await?;
        let properties = properties.unwrap();
        let local_name = properties
            .local_name
            .unwrap_or_else(|| String::from("(unknown)"));
        let address = properties.address.to_string();
        print!(
            "Found device [{}]: {}. ", address, local_name
        );

        // Check if it is the named peripheral
        if (device_name.is_not_empty() && local_name != device_name) || (device_address.is_not_empty() && address != device_address) {
            if verbosity > 0 {
                println!("Skipping.");
            } else {
                println!();
            }
            continue;
        } else {
            device_found = true;
        }

        // Connect to device
        if !is_connected {
            println!("Connecting... ");
            if let Err(err) = peripheral.connect().await {
                eprintln!("Error connecting to peripheral ({}).", err);
                continue;
            }
        }
        let is_connected = peripheral.is_connected().await?;
        if verbosity > 0{
            println!("Connected to {:}...", &local_name);
        }

        // Discover services
        peripheral.discover_services().await?;
        if verbosity > 0{
            println!("Discovering services on {:}...", &local_name);
        }

        if verbosity > 0 {    // Display debug dump of services and readable characteristics
            for service in peripheral.services() {
                println!("Service {}    primary: {}", service.uuid.to_short_string(), service.primary);
                // Print the readable chars to screen
                for characteristic in service.characteristics {
                    print!("        {}", characteristic.uuid.to_short_string());
                    println!("    {:?}", characteristic.properties);
                    if characteristic.properties.contains(CharPropFlags::READ) {
                        let data = peripheral.read(&characteristic).await?;
                        print!("        {}    DATA READ        ", characteristic.uuid.to_short_string());
                        let mut s: String = String::new();
                        for zx in data.iter() {
                            let x = *zx;
                            print!("{:02x} ", x);
                            if x > 31 && x < 127 {
                                let c = x as char;
                                s.push(c);
                            } else {
                                s.push('.');
                            }
                        }
                        print!("    '{}'", s);
                        if data.len() == 1 {
                            print!("    {}", u8::from_le_bytes([data[0]]));
                        } else if data.len() == 2 {
                            print!("    {}", u16::from_le_bytes([data[0], data[1]]));
                        } else if data.len() == 4 {
                            print!("    {}", u32::from_le_bytes([data[0], data[1], data[2], data[3]]));
                        }
                        println!();
                    }
                }
            }
        }
    
        // Check that this looks like a DaFit watch

        // Check for all required services
        let services = peripheral.services();
        let s_uuids: Vec<Uuid> = services.iter().map(|s| s.uuid).collect();
        if !(s_uuids.contains(&SU_DEVINFO) && s_uuids.contains(&SU_FEEA) && s_uuids.contains(&SU_BATTERY)) {
            println!("This doesn't look like a compatible device.");
            continue;
        }
        
        // Check for all required characteristics
        let chars = peripheral.characteristics();                
        let required_chars = vec!(CU_SOFTREV, CU_SERIALNUM, CU_MANUFACTURER, CU_BATTERY, CU_NOTIFY, CU_SEND, CU_SENDFILE);
        for rc in required_chars {
            if !chars.iter().any(|c| c.uuid==rc) {
                println!("Device does not have all required characteristics.");
                continue;
            }
        }

        // Read some device info
        let mut c: &Characteristic;
        let mut data: Vec<u8>;

        c = chars.iter().find(|c| c.uuid == CU_SOFTREV).unwrap();
        data = peripheral.read(c).await?;                    
        let software_revision = String::from_utf8_lossy(&data).into_owned();

        c = chars.iter().find(|c| c.uuid == CU_SERIALNUM).unwrap();
        data = peripheral.read(c).await?;
        let serial_number = String::from_utf8_lossy(&data).into_owned();

        c = chars.iter().find(|c| c.uuid == CU_MANUFACTURER).unwrap();
        data = peripheral.read(c).await?;
        let manufacturer = String::from_utf8_lossy(&data).into_owned();

        c = chars.iter().find(|c| c.uuid == CU_BATTERY).unwrap();
        data = peripheral.read(c).await?;
        let battery_level = format!("{}", data[0]);

        println!("Software Revision: {}", software_revision);
        println!("Serial Number:     {}", serial_number);
        println!("Manufacturer:      {}", manufacturer);
        println!("Battery Level:     {}", battery_level);

        if manufacturer != "MOYOUNG-V2" {
            println!("This doesn't look like a compatible device.");
            continue;
        }

        device_found = true;

        // Subscribe to notifications
        let cnotify = chars.iter().find(|c| c.uuid == CU_NOTIFY).unwrap();
        peripheral.subscribe(cnotify).await?; // clippy removed &
        let mut notification_stream = peripheral.notifications().await?;
        
        let csend = chars.iter().find(|c| c.uuid == CU_SEND).unwrap();
        let csendfile = chars.iter().find(|c| c.uuid == CU_SENDFILE).unwrap();

        // If we have filedata, send it
        if filedata.is_not_empty() && mode == Mode::Upload {             
            const CHUNKSIZE: usize = 244;
            println!("Sending watch face...");
            std::io::stdout().flush().unwrap();

            // Send the prep command
            data = vec![ 0xfe, 0xea, 0x20, 0x09, 0x74 ];
            let fsize: u32 = filedata.len() as u32;
            data.extend_from_slice(&fsize.to_be_bytes());
            if verbosity > 0 {
                println!("SEND: {}", data.iter().map(|c| format!("{:02x} ", c)).collect::<String>());
            }
            peripheral.write(csend, &data, WriteType::WithoutResponse).await?;

            let mut expected_num: usize = 0;

            // Loop until we receive an 'all done' message
            let mut finished: bool = false;
            while !finished {                       
                if verbosity > 0 {
                    println!("Waiting for notification...");
                }
                let data = match notification_stream.next().await {
                    Some(x) => x.value,
                    _ => { 
                        println!("ERROR: reading data from notification"); 
                        break;
                    },
                };

                if verbosity > 0 {
                    println!("RECV: {}", data.iter().map(|c| format!("{:02x} ", c)).collect::<String>());
                } 

                if data[0..5] == [ 0xfe, 0xea, 0x20, 0x09, 0x74 ] {             // All done
                    print!("\x0D{:<5.2} % ", 100);  // 100%
                    let checksum: u32 = u32::from_be_bytes(data[5..=8 ].try_into()?);
                    println!("All data recived by watch. Checksum: {:08x} ({})", checksum, checksum as i32);

                    peripheral.write(csend, &[ 0xfe, 0xea, 0x20, 0x09, 0x74, 0x00, 0x00, 0x00, 0x00 ], WriteType::WithoutResponse).await?;
                    finished = true;
                } else if data[0..5] == [ 0xfe, 0xea, 0x20, 0x07, 0x74 ] {      // Ready for chunk
                    let chunknum: usize = (u16::from_be_bytes(data[5..=6].try_into().unwrap())) as usize;                            
                    let startidx: usize = chunknum * CHUNKSIZE;
                    let mut endidx: usize = startidx + CHUNKSIZE;

                    if chunknum != expected_num {
                        println!("WARNING: Expected request for chunk {}, got request for chunk {}", expected_num, chunknum);
                    }
                    expected_num = chunknum + 1;
                    if endidx > fsize as usize {
                        endidx = fsize as usize;
                    }
                    if verbosity > 0 {
                        println!("Sending chunk #{}", chunknum);
                    } else {
                        let pc: f64 = (chunknum * CHUNKSIZE * 100) as f64/ (fsize as f64);
                        print!("\x0D{:<5.2} % ", pc);
                    }
                    io::stdout().flush().unwrap();
                    peripheral.write(csendfile, &filedata[startidx..endidx], WriteType::WithoutResponse).await?;  // Send requested chunk
                } else {
                    println!("WARNING: Unexpected data from watch!");
                }
            }
            if finished {
                println!("File send finished!");
                // Switch to watch face feea2006190d --- number 13, the custom watch face we stored at file 0x74. File stored at 0x6e is in watch face #6.
                peripheral.write(csend, &[0xfe, 0xea, 0x20, 0x06, 0x19, 0x0d ], WriteType::WithoutResponse).await?;  // send requested chunk
            }
            time::sleep(Duration::from_millis(1000)).await;
        }

        if is_connected {
            println!("Disconnecting from {}.", &local_name);
            peripheral
                .disconnect()
                .await
                .expect("Error disconnecting from BLE peripheral!");
        }
    }
    Ok(())
}