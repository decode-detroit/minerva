import React from 'react';
import logoWide from './logo_wide.png';
import { EditMenu } from './components/Menus.js';
import { ViewArea } from './components/EditArea.js';
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
      saved: true,
      configFile: "",
      randomCss: Math.floor(Math.random() * 1000000), // Scramble the css file name
    }

    // Bind the various functions
    this.saveModifications = this.saveModifications.bind(this);
    this.saveStyle = this.saveStyle.bind(this);
    this.saveFile = this.saveFile.bind(this);
    this.handleFileChange = this.handleFileChange.bind(this);
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
        configFile: json.path.path, // FIXME Allow for more subtlety
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
            <EditMenu saved={this.state.saved} filename={this.state.configFile} handleFileChange={this.handleFileChange} saveFile={this.saveFile} />
          </div>
          <ViewArea saveModifications={this.saveModifications} saveStyle={this.saveStyle} />
        </div>
      </>
    )
  }
}

export default App;
