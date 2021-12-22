import React from 'react';
import logoWide from './logo_wide.png';
import { RunMenu } from './components/Menus.js';
import { ViewArea } from './components/RunArea.js';
import { saveEdits, saveStyle, saveConfig } from './components/Functions';
import './App.css';

// The top level class
export class App extends React.PureComponent {  
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
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
      keyMap: {},
      randomCss: Math.floor(Math.random() * 1000000), // Scramble the css file name
    }

    // Bind the various functions
    this.processUpdate = this.processUpdate.bind(this);
  }

  // On render, connect the websocket listener
  async componentDidMount() {
    // Connect the websocket for updates
    this.socket = new WebSocket('ws://' + window.location.host + '/listen')
    this.socket.onmessage = this.processUpdate.bind(this)
  }
  
  // Listen for updates from the server
  async processUpdate(update) {
    // Parse the incoming update (must be parsed with JSON.parse as it arrives as a string)
    const data = JSON.parse(update.data);

    // Switch based on the interface update
    // Change the display settings
    if (data.hasOwnProperty(`changeSettings`)) {
      // Extract the update
      let update = data[`changeSettings`][`displaySetting`];

      // If the request is to switch to fullscreen
      if (update.hasOwnProperty(`fullScreen`)) {
        // Switch to or leave fullscreen
        if (update.fullScreen) {
          document.documentElement.requestFullscreen(); // FIXME need to prompt the user
        } else {
          document.exitFullscreen();
        }
      
      // If the request is to switch debug mode
      } else if (update.hasOwnProperty(`debugMode`)) {
        this.setState({
          debugMode: update.debugMode,
        });
      
      // If the request is to switch font mode
      } else if (update.hasOwnProperty(`largeFont`)) {
        this.setState({
          largeFont: update.largeFont,
        });

      // If the request is to switch contrast mode
      } else if (update.hasOwnProperty(`highContrast`)) {
        this.setState({
          highContrast: update.highContrast,
        });
      }
    
    // Post a current event to the status bar
    } else if (data.hasOwnProperty(`notify`)) {
      this.setState({
        notice: data[`notify`][`message`],
      }); 

    // Update the available scenes and full status in the window
    } else if (data.hasOwnProperty(`updateConfig`)) {
      this.setState({
        scenes: data[`updateConfig`][`scenes`],
        fullStatus: data[`updateConfig`][`fullStatus`],
      });
    
    // Refresh the entire button window with a new window
    } else if (data.hasOwnProperty(`updateWindow`)) {
      this.setState({
        currentScene: data[`updateWindow`][`currentScene`],
        currentItems: data[`updateWindow`][`currentItems`],
        keyMap: data[`updateWindow`][`keyMap`],
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

    // Update the event timeline
    } else if (data.hasOwnProperty(`updateTimeline`)) {
      this.setState({
        timelineEvents: data[`updateTimeline`][`events`],
      });
    }
  }

  // Render the complete application
  render() {
    return (
      <>
        <link id="userStyles" rel="stylesheet" href={`/getStyles/${this.state.randomCss}.css`} />
        <div className="app">
          <div className="header">
            <img src={logoWide} className="logo" alt="logo" />
          </div>
          <ViewArea currentScene={this.state.currentScene} currentItems={this.state.currentItems} />
        </div>
      </>
    )
  }
}

export default App;
