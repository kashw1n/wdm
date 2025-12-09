# WDM (Web Download Manager)

![WDM Logo](public/logo-readme.png)

A modern, high-performance download manager built with **Tauri** and **React**. WDM combines the native performance of Rust with a sleek, reactive user interface to handle both standard file downloads and video content from popular platforms.


## 🚀 Features

### Core Functionality
*   **High-Speed Downloading:** Utilizes multi-threaded chunked downloading (up to 32 connections) to maximize bandwidth usage.
*   **Resumable Downloads:** Pause and resume functionality for interrupted downloads.
*   **Persistence:** Automatically saves download history and progress state.
*   **Smart File Handling:** Auto-renaming for duplicate files and `.part` file handling for incomplete downloads.

### 🎥 Video Support
*   **Integrated Video Downloader:** Seamlessly download videos from YouTube and other supported platforms.
*   **Format Selection:** Choose specific resolutions and formats before downloading.
*   **Automatic Setup:** Automatically handles the installation and management of the `yt-dlp` binary required for video processing.
*   **Real-time Feedback:** Accurate progress bars, download speed, and ETA for video tasks.

### 🎨 User Interface
*   **Modern Dark Theme:** A polished, eye-friendly dark interface designed with **Tailwind CSS v4**.
*   **Responsive Design:** Adapts to various window sizes.
*   **Real-time Metrics:** Live updates for download speeds, remaining time, and file sizes.

### ⚙️ Customization
*   **Connection Limits:** Configure the number of concurrent connections per download.
*   **Speed Limiting:** Set global bandwidth limits.
*   **Custom Directories:** Choose your preferred download location.

## 🛠️ Tech Stack

*   **Frontend:** React, TypeScript, Tailwind CSS
*   **Backend:** Rust, Tauri, Tokio (Async Runtime), Reqwest
*   **Tools:** `yt-dlp` (embedded/managed for video extraction)

## 📦 Getting Started

### Prerequisites
*   [Node.js](https://nodejs.org/) (v16+)
*   [Rust](https://www.rust-lang.org/) (latest stable)

### Installation

1.  **Clone the repository**
    ```bash
    git clone https://github.com/yourusername/wdm.git
    cd wdm
    ```

2.  **Install frontend dependencies**
    ```bash
    npm install
    ```

3.  **Run in development mode**
    ```bash
    npm run tauri dev
    ```

4.  **Build for production**
    ```bash
    npm run tauri build
    ```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.