import Foundation
import Darwin

enum CoreType: Int, CaseIterable {
    case unknown = -1
    case efficiency = 1
    case performance = 2
}

struct CoreInfo {
    let id: Int32
    let type: CoreType
    let clusterId: Int32
}

struct CPUCoreTicks {
    let coreId: Int32
    let userTicks: Int32
    let systemTicks: Int32
    let niceTicks: Int32
    let idleTicks: Int32
}

struct CPUMetrics {
    let cores: [CoreInfo]
    let coreTickCounts: [CPUCoreTicks]
    let totalCores: Int32
}

func detectCoreTopology() -> [CoreInfo] {
    var cores: [CoreInfo] = []
    
    // Search IOKit registry for CPU topology - this method works reliably on Apple Silicon
    var iterator = io_iterator_t()
    let result = IOServiceGetMatchingServices(kIOMainPortDefault, IOServiceMatching("IOPlatformExpertDevice"), &iterator)
    if result != kIOReturnSuccess {
        return cores
    }
    
    while case let service = IOIteratorNext(iterator), service != 0 {
        searchServiceRecursively(service, depth: 0, cores: &cores)
        IOObjectRelease(service)
    }
    IOObjectRelease(iterator)
    
    cores.sort { $0.id < $1.id }
    return cores
}

func searchServiceRecursively(_ service: io_registry_entry_t, depth: Int, cores: inout [CoreInfo]) {
    var name: io_name_t = (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
    IORegistryEntryGetName(service, &name)
    let nameStr = withUnsafePointer(to: &name) {
        $0.withMemoryRebound(to: CChar.self, capacity: 128) {
            String(validatingUTF8: $0) ?? ""
        }
    }
    
    // Look for CPU entries (cpu0, cpu1, etc.)
    if nameStr.hasPrefix("cpu") && nameStr.count > 3 {
        examineEntry(service, name: nameStr, cores: &cores)
    }
    
    // Recurse into children (limit depth to avoid infinite loops)
    if depth < 3 {
        var iterator = io_iterator_t()
        if IORegistryEntryGetChildIterator(service, kIOServicePlane, &iterator) == kIOReturnSuccess {
            var child = IOIteratorNext(iterator)
            while child != 0 {
                searchServiceRecursively(child, depth: depth + 1, cores: &cores)
                IOObjectRelease(child)
                child = IOIteratorNext(iterator)
            }
            IOObjectRelease(iterator)
        }
    }
}

func examineEntry(_ entry: io_registry_entry_t, name: String, cores: inout [CoreInfo]) {
    var props: Unmanaged<CFMutableDictionary>?
    if IORegistryEntryCreateCFProperties(entry, &props, kCFAllocatorDefault, 0) == kIOReturnSuccess,
       let properties = props?.takeRetainedValue() as? [String: Any] {
        
        processCPUEntry(properties, name: name, cores: &cores)
    }
}

func processCPUEntry(_ properties: [String: Any], name: String, cores: inout [CoreInfo]) {
    var coreType: CoreType = .unknown
    
    // Extract cluster type (E for efficiency, P for performance)
    if let rawType = properties["cluster-type"] as? Data,
       let type = String(data: rawType, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines),
       !type.isEmpty {
        
        let firstChar = type.first!
        switch firstChar {
        case "E":
            coreType = .efficiency
        case "P":
            coreType = .performance
        default:
            coreType = .unknown
        }
    }
    
    // Extract CPU ID and cluster ID
    let cpuId = (properties["cpu-id"] as? Data)?.withUnsafeBytes { pointer in
        return pointer.load(as: Int32.self)
    } ?? -1
    
    let clusterId = (properties["cluster-id"] as? Data)?.withUnsafeBytes { pointer in
        return pointer.load(as: Int32.self)
    } ?? -1
    
    // Only add valid cores
    if cpuId >= 0 && clusterId >= 0 {
        cores.append(CoreInfo(id: cpuId, type: coreType, clusterId: clusterId))
    }
}

func getCPUCount() -> Int32 {
    var mib = [CTL_HW, HW_NCPU]
    var numCPUs: uint = 0
    var sizeOfNumCPUs: size_t = MemoryLayout<uint>.size
    let status = sysctl(&mib, 2, &numCPUs, &sizeOfNumCPUs, nil, 0)
    return status == 0 ? Int32(numCPUs) : 1
}

func getCurrentCPUTicks(numCPUs: Int32) -> [CPUCoreTicks] {
    var cpuInfo: processor_info_array_t!
    var numCpuInfo: mach_msg_type_number_t = 0
    var numCPUsU: natural_t = 0
    var coreTickCounts: [CPUCoreTicks] = []
    
    let result = host_processor_info(mach_host_self(), PROCESSOR_CPU_LOAD_INFO, &numCPUsU, &cpuInfo, &numCpuInfo)
    if result == KERN_SUCCESS {
        for i in 0..<numCPUs {
            // Get raw tick counts (stateless - no calculations)
            let user = cpuInfo[Int(CPU_STATE_MAX * i + CPU_STATE_USER)]
            let system = cpuInfo[Int(CPU_STATE_MAX * i + CPU_STATE_SYSTEM)]
            let nice = cpuInfo[Int(CPU_STATE_MAX * i + CPU_STATE_NICE)]
            let idle = cpuInfo[Int(CPU_STATE_MAX * i + CPU_STATE_IDLE)]
            
            coreTickCounts.append(CPUCoreTicks(
                coreId: i,
                userTicks: user,
                systemTicks: system,
                niceTicks: nice,
                idleTicks: idle
            ))
        }
        
        // Clean up memory
        let cpuInfoSize = MemoryLayout<integer_t>.stride * Int(numCpuInfo)
        vm_deallocate(mach_task_self_, vm_address_t(bitPattern: cpuInfo), vm_size_t(cpuInfoSize))
    }
    
    return coreTickCounts
}

func collectCPUMetrics() -> CPUMetrics {
    let cores = detectCoreTopology()
    let numCPUs = getCPUCount()
    let coreTickCounts = getCurrentCPUTicks(numCPUs: numCPUs)
    
    return CPUMetrics(cores: cores, coreTickCounts: coreTickCounts, totalCores: numCPUs)
}

// C-style function for FFI
@_cdecl("collect_cpu_metrics_json")
func collectCPUMetricsJSON() -> UnsafePointer<CChar>? {
    let metrics = collectCPUMetrics()
    
    var jsonData: [String: Any] = [
        "total_cores": metrics.totalCores,
        "cores": [],
        "tick_counts": []
    ]
    
    // Core topology info
    var coresArray: [[String: Any]] = []
    for core in metrics.cores {
        let coreData: [String: Any] = [
            "id": core.id,
            "type": core.type.rawValue,
            "cluster_id": core.clusterId
        ]
        coresArray.append(coreData)
    }
    jsonData["cores"] = coresArray
    
    // Raw tick counts for each core
    var tickCountsArray: [[String: Any]] = []
    for tickCounts in metrics.coreTickCounts {
        let tickData: [String: Any] = [
            "core_id": tickCounts.coreId,
            "user_ticks": tickCounts.userTicks,
            "system_ticks": tickCounts.systemTicks,
            "nice_ticks": tickCounts.niceTicks,
            "idle_ticks": tickCounts.idleTicks
        ]
        tickCountsArray.append(tickData)
    }
    jsonData["tick_counts"] = tickCountsArray
    
    do {
        let jsonDataSerialized = try JSONSerialization.data(withJSONObject: jsonData, options: [])
        if let jsonString = String(data: jsonDataSerialized, encoding: .utf8) {
            return UnsafePointer(strdup(jsonString))
        }
    } catch {
        return nil
    }
    
    return nil
}

@_cdecl("free_string")
func freeString(_ ptr: UnsafePointer<CChar>?) {
    if let ptr = ptr {
        free(UnsafeMutableRawPointer(mutating: ptr))
    }
}