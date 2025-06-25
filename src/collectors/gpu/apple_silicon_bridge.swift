import Foundation
import IOKit

struct GPUInfo {
    let name: String
    let utilization: Double?
}

func fetchIOService(_ name: String) -> [NSDictionary]? {
    var iterator: io_iterator_t = io_iterator_t()
    var obj: io_registry_entry_t = 1
    var list: [NSDictionary] = []
    
    let result = IOServiceGetMatchingServices(kIOMainPortDefault, IOServiceMatching(name), &iterator)
    if result != kIOReturnSuccess {
        return nil
    }
    
    while obj != 0 {
        obj = IOIteratorNext(iterator)
        if let props = getIOProperties(obj) {
            list.append(props)
        }
        IOObjectRelease(obj)
    }
    IOObjectRelease(iterator)
    
    return list.isEmpty ? nil : list
}

func getIOProperties(_ entry: io_registry_entry_t) -> NSDictionary? {
    var properties: Unmanaged<CFMutableDictionary>? = nil
    
    if IORegistryEntryCreateCFProperties(entry, &properties, kCFAllocatorDefault, 0) != kIOReturnSuccess {
        return nil
    }
    
    defer {
        properties?.release()
    }
    
    return properties?.takeUnretainedValue()
}

func collectGPUMetrics() -> [GPUInfo] {
    guard let accelerators = fetchIOService("IOAccelerator") else {
        return []
    }
    
    var gpuInfos: [GPUInfo] = []
    
    for (index, accelerator) in accelerators.enumerated() {
        guard let ioClass = accelerator.object(forKey: "IOClass") as? String else {
            continue
        }
        
        guard let stats = accelerator["PerformanceStatistics"] as? [String: Any] else {
            continue
        }
        
        // Get GPU name and type
        let ioClassLower = ioClass.lowercased()
        var gpuName = "Unknown GPU"
        
        if ioClassLower == "nvaccelerator" || ioClassLower.contains("nvidia") {
            gpuName = "NVIDIA GPU"
        } else if ioClassLower.contains("amd") {
            gpuName = "AMD GPU"
        } else if ioClassLower.contains("intel") {
            gpuName = "Intel GPU"
        } else if ioClassLower.contains("agx") {
            gpuName = stats["model"] as? String ?? "Apple Silicon GPU"
        }
        
        // Add index if multiple GPUs of same type
        if accelerators.count > 1 {
            gpuName += " #\(index)"
        }
        
        // Get utilization percentage
        let utilization: Int? = stats["Device Utilization %"] as? Int ?? stats["GPU Activity(%)"] as? Int
        let utilizationPercent = utilization != nil ? Double(utilization!) / 100.0 : nil
        
        gpuInfos.append(GPUInfo(name: gpuName, utilization: utilizationPercent))
    }
    
    return gpuInfos
}

// C-style function for FFI
@_cdecl("collect_gpu_metrics_json")
func collectGPUMetricsJSON() -> UnsafePointer<CChar>? {
    let gpuInfos = collectGPUMetrics()
    
    var jsonArray: [[String: Any]] = []
    
    for gpu in gpuInfos {
        var jsonGPU: [String: Any] = [
            "name": gpu.name
        ]
        
        if let utilization = gpu.utilization {
            jsonGPU["utilization"] = utilization
        }
        
        jsonArray.append(jsonGPU)
    }
    
    do {
        let jsonData = try JSONSerialization.data(withJSONObject: jsonArray, options: [])
        if let jsonString = String(data: jsonData, encoding: .utf8) {
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