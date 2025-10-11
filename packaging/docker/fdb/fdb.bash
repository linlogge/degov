#!/bin/bash

#
# fdb.bash
#
# This source file is part of the FoundationDB open source project
#
# Copyright 2013-2024 Apple Inc. and the FoundationDB project authors
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#

# Function to detect if an IP is IPv6 and format it appropriately
function format_ip_for_port() {
    local ip=$1
    # Check if IP contains a colon (IPv6 indicator)
    if [[ $ip == *":"* ]]; then
        # IPv6 address - wrap in brackets
        echo "[$ip]"
    else
        # IPv4 address - no brackets needed
        echo "$ip"
    fi
}

function create_cluster_file() {
    FDB_CLUSTER_FILE=${FDB_CLUSTER_FILE:-/etc/foundationdb/fdb.cluster}
    mkdir -p "$(dirname $FDB_CLUSTER_FILE)"

    if [[ -n "$FDB_CLUSTER_FILE_CONTENTS" ]]; then
        echo "$FDB_CLUSTER_FILE_CONTENTS" > "$FDB_CLUSTER_FILE"
    elif [[ -n $FDB_COORDINATOR ]]; then
        coordinator_ip=$(dig +short "$FDB_COORDINATOR")
        if [[ -z "$coordinator_ip" ]]; then
            echo "Failed to look up coordinator address for $FDB_COORDINATOR" 1>&2
            exit 1
        fi
        coordinator_port=${FDB_COORDINATOR_PORT:-4500}
        formatted_coordinator_ip=$(format_ip_for_port "$coordinator_ip")
        echo "docker:docker@$formatted_coordinator_ip:$coordinator_port" > "$FDB_CLUSTER_FILE"
    else
        echo "FDB_COORDINATOR environment variable not defined" 1>&2
        exit 1
    fi
}

function create_server_environment() {
    env_file=/var/fdb/.fdbenv

    if [[ "$FDB_NETWORKING_MODE" == "host" ]]; then
        public_ip=127.0.0.1
    elif [[ "$FDB_NETWORKING_MODE" == "container" ]]; then
        # Get the first IP address, handling both IPv4 and IPv6
        public_ip=$(hostname -i | awk '{print $1}')
        
        # Remove any interface suffix like %eth0 from IPv6 addresses
        public_ip="${public_ip%%%*}"
    else
        echo "Unknown FDB Networking mode \"$FDB_NETWORKING_MODE\"" 1>&2
        exit 1
    fi

    # Format IP for use with port
    formatted_public_ip=$(format_ip_for_port "$public_ip")
    
    # Debug output
    echo "DEBUG: Raw IP: $public_ip"
    echo "DEBUG: Formatted IP: $formatted_public_ip"
    
    # Write to env file with proper quoting
    echo "export PUBLIC_IP='$public_ip'" > $env_file
    echo "export FORMATTED_PUBLIC_IP='$formatted_public_ip'" >> $env_file
    
    if [[ -z $FDB_COORDINATOR && -z "$FDB_CLUSTER_FILE_CONTENTS" ]]; then
        FDB_CLUSTER_FILE_CONTENTS="docker:docker@$formatted_public_ip:$FDB_PORT"
    fi

    create_cluster_file
}

create_server_environment
source /var/fdb/.fdbenv
echo "Starting FDB server on $FORMATTED_PUBLIC_IP:$FDB_PORT"
fdbserver --listen_address "[::]:$FDB_PORT" --public_address "$FORMATTED_PUBLIC_IP:$FDB_PORT" \
    --datadir /var/fdb/data --logdir /var/fdb/logs \
    --locality_zoneid="$(hostname)" --locality_machineid="$(hostname)" --class "$FDB_PROCESS_CLASS" --knob_disable_posix_kernel_aio=1

