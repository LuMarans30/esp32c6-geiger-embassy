# ESP2-C6 Geiger

This is a minimal working example of how to detect pulses from a Geiger board (e.g. RadiationD v1.1 CAJOE) using ESP-HAL with Embassy. 

It uses an async task to detect and log pulses without blocking the main function. 
