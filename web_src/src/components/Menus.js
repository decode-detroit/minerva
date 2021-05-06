import React from 'react';
import { asyncForEach, stopPropogation } from './functions';

// A menu for the edit items
export class EditMenu extends React.PureComponent {  
  // Render the edit menu
  render() {
    return (
      <div className="menu-item">
        <div>Minerva | Edit Mode</div>
      </div>
    );
  }
}

// An add menu with a search bar
export class AddMenu extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      value: null,
      unfiltered: [],
      filtered: [],
    }
 
    // Bind the various functions
    this.handleChange = this.handleChange.bind(this);
  }

  // On load, get the list of potential items
  async componentDidMount() {
    try {
      // Fetch all items and process the response
      let response = await fetch(`allItems`);
      const json = await response.json();

      // If the response is valid
      if (json.items.isValid) {
        // Get the detail of each item
        let list = [];
        await asyncForEach(json.items.items, async (item) => {
          // Fetch the description of the item
          response = await fetch(`getItem/${item.id}`);
          const json2 = await response.json();

          // If valid, save the id and description
          if (json2.item.isValid) {
            list.push({
              id: item.id,
              description: json2.item.itemPair.description,
            });
          }
        });

        // Save the result to the state
        this.setState({
          unfiltered: list,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // Function to handle typing the input
  handleChange(e) {
    // Grab the value
    let value = e.target.value;
    
    // Calculate the filtered list
    let filtered = this.state.unfiltered.filter(item => {
      return (item.description.toLowerCase().includes(value.toLowerCase()));
    });
    
    this.setState({
      value: e.target.value,
      filtered: filtered,
    });
  }
 
  // Return the completed box
  render() {
    // Compose the filtered items into a visible list
    let list = this.state.filtered.map((item) => <div className="divButton" onClick={() => {this.props.addItem(item.id)}}>{item.description}</div>)
    
    // Return the box
    return (
      <div className="addMenu" style={{ left: `${this.props.left}px`, top: `${this.props.top - 40}px` }} onMouseDown={stopPropogation}>
        <div className="title">Add Item</div>
        <input className="searchBar" type="text" placeholder="Type to search ..." value={this.state.value} onInput={this.handleChange}></input>
        <div className="verticalScroll">
          <div>{list}</div>
        </div>
      </div>
    );
  }
}

// A menu for the scene selection
export class SceneMenu extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props and set initial state
    super(props);
    this.state = {
      sceneList: [{id: -1, description: "Any"}],
      flag: true,
    }

    // Bind functions
    this.handleChange = this.handleChange.bind(this);
  }

  // On render, pull the full scene list
  async componentDidMount() {
    try {
      // Fetch all scenes and process the response
      let response = await fetch(`/allScenes`);
      const json = await response.json();

      // If the response is valid
      if (json.items.isValid) {
        // Get the detail of each item
        let sceneList = [{id: -1, description: "Any"}];
        await asyncForEach(json.items.items, async (item) => {
          // Fetch the description of the item
          response = await fetch(`/getItem/${item.id}`);
          const json2 = await response.json();

          // If valid, save the id and description
          if (json2.item.isValid) {
            sceneList.push({
              id: item.id,
              description: json2.item.itemPair.description,
            });
          }
        });

        // Save the result to the state and prompt refresh
        this.setState({
          sceneList: sceneList,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // Control the view of the component
  handleChange(e) {
    // Pass the change upstream
    this.props.changeScene(e.target.value);
  }
  
  // Render the scene menu
  render() {
    // Compose the list of options
    let options = this.state.sceneList.map((scene) => <option key={scene.id.toString()} value={scene.id}>{scene.description}</option>);
    options.sort((first, second) => { return first.id - second.id } );

    // Return the complete menu
    return (
      <div className="sceneMenu">
        <div className="title">
          Show Items From Scene:
        </div>
        <select className="select" value={this.props.value} onChange={this.handleChange}>
          {options}
        </select>
      </div>
    );
  }
}
