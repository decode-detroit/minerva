import React from 'react';
import { HeaderMenu } from './components/Menus.js';
import { ViewArea } from './components/EditArea.js';
import { saveEdits, saveStyle, openConfig, saveConfig } from './components/Functions';
import { FullscreenDialog } from './components/Dialogs.js';
import './App.css';

// The top level class
export class App extends React.PureComponent {  
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      connectionActive: false,
      saved: true,
      configFile: "",
      randomCss: Math.floor(Math.random() * 1000000), // Scramble the css file name
    }

    // Bind the various functions
    this.saveModifications = this.saveModifications.bind(this);
    this.saveStyle = this.saveStyle.bind(this);
    this.openFile = this.openFile.bind(this);
    this.saveFile = this.saveFile.bind(this);
    this.handleFileChange = this.handleFileChange.bind(this);
    this.connectSocket = this.connectSocket.bind(this);
    this.processUpdate = this.processUpdate(this);

    // Save variables (not based on state)
    this.socket = null;
    this.socketInterval = null;
  }

  // Save any modification to the configuration
  saveModifications(modifications) {
    // Save the modifications
    saveEdits(modifications);

    // Mark changes as unsaved
    this.setState({
      saved: false,
    });
  }

  // Save any style changes to the configuration
  saveStyle(selector, rule) {
    // Save the style
    saveStyle(selector, rule);
    
    // Mark changes as unsaved
    this.setState({
      saved: false,
    });
  }

  // Open the selected configuration file
  openFile() {
    // Save the configuration with the current filename
    openConfig(this.state.configFile);

    // Update the save state and clear the rules
    this.setState({
      saved: true,
    });
  }

  // Save the configuration to the current filename
  saveFile() {
    // Save the configuration with the current filename
    saveConfig(this.state.configFile);

    // Update the save state and clear the rules
    this.setState({
      saved: true,
    });
  }

  // Function to handle new text in the input
  handleFileChange(e) {
    // Save the new value as the filename
    this.setState({
      configFile: e.target.value,
      saved: false,
    });
  }

  // On render, pull the configuration file path
  async componentDidMount() {
    // Retrieve the configuation path
    const response = await fetch(`getConfigPath`);
    const json = await response.json();

    // If valid, save configuration
    if (json.path.isValid) {
      this.setState({
        configFile: json.path.path,
      });
    }

    // Connect the listening socket
    this.connectSocket();
  }
  
  // A helper function to connect the websocket
  async connectSocket() {
    // Try to connect the websocket for updates
    this.socket = await new WebSocket('ws://' + window.location.host + '/listen');
    
    // Connect the message listener
    this.socket.onmessage = this.processUpdate;

    // If the socket opens
    this.socket.onopen = ((_) => {

      // Clear the old timeout if it exists
      if (this.socketInterval) {
        clearTimeout(this.socketInterval);
        this.socketInterval = null;
      }

      // Mark the server as available 
      this.setState({
        connectionActive: true,
      });
    });

    // If the socket closes
    this.socket.onclose = ((_) => {
      // If the interval is inactive
      if (!this.socketInterval) {
        // Mark the server as unavailable 
        this.setState({
          connectionActive: false,
        });

        // Try once every five seconds to restart the connection
        this.socketInterval = setInterval(() => {
          this.connectSocket();
        }, 5000);
      }
    });
  }

  // Function to handle updates from the web socket
  processUpdate() {}

  // Function to close the program and mark it as inactive
  async closeMinerva() {
    // Close the program
    fetch(`/close`, {
      method: 'POST',
      headers: {
          'Content-Type': 'application/json',
      },
    });
  }

  // Render the complete application
  render() {
    return (
      <>
        <link id="userStyles" rel="stylesheet" href={`/getStyles/${this.state.randomCss}.css`} />
        <div className="app">
          <HeaderMenu closeMinerva={this.closeMinerva} saved={this.state.saved} filename={this.state.configFile} handleFileChange={this.handleFileChange} openFile={this.openFile} saveFile={this.saveFile} />
          <ViewArea saveModifications={this.saveModifications} saveStyle={this.saveStyle} filename={this.state.configFile} handleFileChange={this.handleFileChange}/>
        </div>
        {!this.state.connectionActive && <FullscreenDialog dialogType="error" dialogTitle="Minerva Is Unavailable" dialogMessage="Minerva is closed or currently inaccessible. Please restart the program." dialogRetry={this.connectSocket}/>}
      </>
    )
  }
}

export default App;
