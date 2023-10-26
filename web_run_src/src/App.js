import React from 'react';
import { HeaderMenu, FooterMenu } from './components/Menus.js';
import { ViewArea } from './components/RunArea.js';
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
      debugMode: false,
      largeFont: false,
      highContrast: false,
      notice: "",
      notifications: [],
      timelineEvents: [],
      scenes: [],
      fullStatus: {},
      currentScene: {},
      currentItems: [],
      randomCss: Math.floor(Math.random() * 1000000), // Scramble the css file name
    }

    // Bind the various functions
    this.connectSocket = this.connectSocket.bind(this);
    this.processUpdate = this.processUpdate.bind(this);
    this.closeMinerva = this.closeMinerva.bind(this);

    // Save variables (not based on state)
    this.socket = null;
    this.socketInterval = null;
  }

  // On render, connect the websocket listener
  async componentDidMount() {
    // Try to connect the socket
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

  // Listen for updates from the server
  async processUpdate(update) {
    // Parse the incoming update (must be parsed with JSON.parse as it arrives as a string)
    const data = JSON.parse(update.data);

    // Switch based on the interface update
    // Update the current scene and status
    if (data.hasOwnProperty(`currentSceneAndStatus`)) {
      // Save the curent scene // FIXME also save the status
      this.setState({
        currentScene: data[`currentSceneAndStatus`][`currentScene`],
      });

    // Post a current event to the status bar
    } else if (data.hasOwnProperty(`notify`)) {
      this.setState({
        notice: {
          message: data[`notify`][`message`],
          time: new Date(),
        }
      });
    
    // Refresh the entire display
    } else if (data.hasOwnProperty(`refreshAll`)) {
      window.location.reload(false); // FIXME a bit of a clumsy solution
    
    // Update the current scene
    } else if (data.hasOwnProperty(`updateScene`)) {
      // Save the curent scene
      this.setState({
        currentScene: data[`updateScene`][`currentScene`],
      });

    // Update the current state of a particular status
    } else if (data.hasOwnProperty(`updateStatus`)) {
      this.setState((prevState) => {
        // Update the particular status
        let newStatus = {...prevState.fullStatus};
        newStatus[`${data['updateStatus']['statusId']}`] = data[`updateStatus`][`newState`];
        
        // Update the full status
        return {
          fullStatus: newStatus,
        };
      });
    
    // Update the current notifications
    } else if (data.hasOwnProperty(`updateNotifications`)) {
      this.setState({
        notifications: data[`updateNotifications`][`notifications`],
      });

      // FIXME Need a better place for this than the commandline
      console.log(`Notifications`);
      console.log(data[`updateNotifications`][`notifications`]);

    // Update the event timeline
    } else if (data.hasOwnProperty(`updateTimeline`)) {
      this.setState({
        timelineEvents: data[`updateTimeline`][`events`],
      });

      // FIXME Need to show the timeline somewhere
      console.log(`Timeline`);
      console.log(data[`updateTimeline`][`events`]);
    }
  }

  // Function to close the program and mark it as inactive
  async closeMinerva() {
    // Close the program
    fetch(`/close`, {
      method: 'POST',
      headers: {
          'Content-Type': 'application/json',
      },
    });

    // Mark the program as closed
    this.setState({
      connectionActive: false,
    });
  }

  // Render the complete application
  render() {
    return (
      <>
        <link id="userStyles" rel="stylesheet" href={`/getStyles/${this.state.randomCss}.css`} />
        <div className="app">
          <HeaderMenu closeMinerva={this.closeMinerva} currentScene={this.state.currentScene}/>
          <ViewArea currentScene={this.state.currentScene} />
          <FooterMenu notice={this.state.notice} />
        </div>
        {!this.state.connectionActive && <FullscreenDialog dialogType="error" dialogTitle="Minerva Is Unavailable" dialogMessage="Minerva is closed or currently inaccessible. Please restart the program."/>}
      </>
    )
  }
}

export default App;
