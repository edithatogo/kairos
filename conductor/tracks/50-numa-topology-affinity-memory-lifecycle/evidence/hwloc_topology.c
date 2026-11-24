// SPDX-License-Identifier: Apache-2.0
// Independent host topology readback through the installed hwloc library.
#include <hwloc.h>
#include <inttypes.h>
#include <stdio.h>

int main(void) {
    hwloc_topology_t topology;
    if (hwloc_topology_init(&topology) != 0) {
        perror("hwloc_topology_init");
        return 1;
    }
    if (hwloc_topology_set_type_filter(topology, HWLOC_OBJ_L1ICACHE,
                                       HWLOC_TYPE_FILTER_KEEP_ALL) != 0) {
        perror("hwloc_topology_set_type_filter");
        hwloc_topology_destroy(topology);
        return 1;
    }
    if (hwloc_topology_load(topology) != 0) {
        perror("hwloc_topology_load");
        hwloc_topology_destroy(topology);
        return 1;
    }

    printf("hwloc_api=0x%08x\n", hwloc_get_api_version());
    const hwloc_obj_type_t types[] = {
        HWLOC_OBJ_NUMANODE, HWLOC_OBJ_PACKAGE, HWLOC_OBJ_CORE, HWLOC_OBJ_PU,
        HWLOC_OBJ_L1CACHE, HWLOC_OBJ_L1ICACHE, HWLOC_OBJ_L2CACHE, HWLOC_OBJ_L3CACHE,
    };
    for (size_t i = 0; i < sizeof(types) / sizeof(types[0]); ++i) {
        printf("%s=%d\n", hwloc_obj_type_string(types[i]),
               hwloc_get_nbobjs_by_type(topology, types[i]));
    }

    int result = 0;
    hwloc_obj_t node = NULL;
    while ((node = hwloc_get_next_obj_by_type(topology, HWLOC_OBJ_NUMANODE, node))) {
        char cpus[256];
        int length = hwloc_bitmap_list_snprintf(cpus, sizeof(cpus), node->cpuset);
        if (length < 0 || (size_t)length >= sizeof(cpus)) {
            fputs("could not describe node CPU list\n", stderr);
            result = 1;
            break;
        }
        printf("node=%u cpus=%s memory_bytes=%" PRIu64 "\n", node->os_index,
               cpus, node->attr->numanode.local_memory);
    }
    hwloc_topology_destroy(topology);
    return result;
}
