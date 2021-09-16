import React from 'react';
import logoWide from './logo_wide.png';
import { EditMenu } from './components/Menus.js';
import { ViewArea } from './components/EditArea.js';
import { saveEdits, saveConfig } from './components/functions';
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
    }

    // Bind the various functions
    this.saveFile = this.saveFile.bind(this);
    this.saveModifications = this.saveModifications.bind(this);
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

  // Save the configuration to the current filename
  saveFile() {
    // Save the configuration with the current filename
    saveConfig(this.state.configFile);

    // Update the save state
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
    if (json.generic.isValid) {
      this.setState({
        configFile: json.generic.message,
      });
    }
  }
  
  // Render the complete application
  render() {
    return (
      <div className="app">
        <div className="header">
          <img src={logoWide} className="logo" alt="logo" />
          <EditMenu saved={this.state.saved} filename={this.state.configFile} handleFileChange={this.handleFileChange} saveFile={this.saveFile} />
        </div>
        <ViewArea saveModifications={this.saveModifications}/>
      </div>
    )
  }
}

export default App;
