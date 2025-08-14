# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Setup

This repository contains two separate projects:

### 1. Swarm Main Project
- **Backend**: `cargo run --bin stack` - Main swarm backend server
- **Frontend**: `cd app && yarn dev` - Main swarm frontend (from repo root)
- **Access**: Frontend available at http://localhost:5173/, login with `admin`/`password`

### 2. Swarm SuperAdmin Project  
- **Backend**: `cargo run --bin super` - SuperAdmin backend server
- **Frontend**: `cd src/bin/super/superapp && yarn dev` - SuperAdmin frontend
- **Access**: SuperAdmin interface for managing multiple swarm instances

### Service Management
- **Stop all services**: `./clear.sh`
- **Stop specific service**: `./stop.sh <service>` (services: jarvis, neo4j, elastic, boltwall, navfiber)
- **Restart services**: `./restart.sh`

### Build Commands
- **Main frontend build**: `cd app && yarn build`
- **Main frontend type check**: `cd app && yarn check`
- **SuperAdmin frontend build**: `cd src/bin/super/superapp && yarn build`
- **SuperAdmin frontend type check**: `cd src/bin/super/superapp && yarn check`
- **Rust build**: `cargo build`
- **Rust specific binary**: `cargo run --bin <binary_name>` (available: stack, super, cln, down, btc, sphinx, tome)

## Architecture Overview

### Backend (Rust)
- **Main Swarm Backend**: `src/bin/stack/mod.rs` - Primary application server using Rocket framework
- **SuperAdmin Backend**: `src/bin/super/mod.rs` - Manages multiple swarm instances and AWS resources
- **Shared Core modules**:
  - `routes.rs` - HTTP API endpoints and WebSocket streams for logs/events
  - `handler.rs` - Command processing and Docker container management
  - `images/` - Docker image definitions for various services (Bitcoin, Lightning, databases, etc.)
  - `conn/` - Connection modules for external services (Bitcoin RPC, Lightning nodes, etc.)
  - `builder.rs` - Stack building and container orchestration
  - `config.rs` - YAML configuration loading/saving

### Frontend (Svelte + TypeScript)
- **Main Swarm Frontend**: `app/src/main.ts` -> `App.svelte`
  - Single-page application with conditional rendering based on auth state
  - `Dashboard.svelte` - Main application interface
  - `auth/Login.svelte` - Authentication component
  - `api/` - Backend API communication modules
- **SuperAdmin Frontend**: `src/bin/super/superapp/src/main.ts` -> `App.svelte`
  - Management interface for multiple swarm instances
  - AWS EC2 instance management and monitoring
  - Remote swarm configuration and container control

### Service Types
The system manages various containerized services defined in `src/images/`:
- **Bitcoin**: Core Lightning (CLN), LND, Bitcoin Core
- **Storage**: Neo4j, Elasticsearch, Redis, MongoDB, PostgreSQL
- **Communication**: Relay, Proxy, MQTT Broker
- **AI/ML**: Jarvis, LLaMA, Second Brain
- **Infrastructure**: Traefik, Cache, NavFiber

### Configuration
- **Stack config**: YAML files in root (e.g., `config.yml`, `sphinx.yml`, `second-brain.yml`)
- **Frontend config**: `app/package.json`, `app/vite.config.ts`
- **Build tools**: Uses Vite for frontend, Cargo for backend

### Authentication & Security
- JWT-based authentication with refresh tokens
- Public key authentication support for admin users
- CORS handling for API requests
- Challenge-based authentication flow

### Real-time Features
- Server-sent events for logs (`/api/logstream`)
- Real-time event streaming (`/api/events`)
- WebSocket-like functionality using EventStream

### Docker Integration
- Heavy use of Bollard for Docker API interaction
- Dynamic container creation and management
- Service health monitoring and auto-restart capabilities
- Volume backup and restoration system