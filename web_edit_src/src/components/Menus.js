import React from 'react';
import logoWide from '.././logo_wide.png';
import { ConfirmButton } from './Buttons';
import { asyncForEach, stopPropogation, switchPort, newConfig } from './Functions';

// A menu pop-up for deleting items
export class DeleteMenu extends React.PureComponent {  
  // Class constructor
  constructor(props) {
    // Collect props and set initial state
    super(props);

    // Set initial state
    this.state = {
      description: "Loading ...",
    };

    // Bind the various functions
    this.updateItem = this.updateItem.bind(this);
    this.deleteItem = this.deleteItem.bind(this);
  }

  // Helper function to update the item information
  async updateItem() {
    try {
      // Fetch the description of the status
      let response = await fetch(`getItem/${this.props.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.isValid) {
        // Save the itemPair
        this.setState({
          description: json.data.item.description,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // Helper function to remove all data associated with an item id
  deleteItem() {
    // Save the change
    let modifications = [{
      removeItem: {
        itemId: { id: this.props.id },
      },
    }];
    this.props.saveModifications(modifications);
  }

  // On initial load, pull item information
  componentDidMount() {
    // Pull the item information
    this.updateItem();
  }

  // Render the edit menu
  render() {
    return (
      <div className="deleteConfirmMenu">
        <div className="title">Are you sure you want to delete this item?</div>
        <div className="description disableSelect">{this.state.description}</div>
        <div className="id disableSelect">({this.props.id})</div>
        <div className="multiButton">
          <div onClick={this.props.closeMenu}>Cancel</div>
          <div className="deleteConfirm" onClick={() => {this.deleteItem(); this.props.afterDelete(); this.props.closeMenu()}}>Confirm</div>
        </div>
      </div>
    );
  }
}

// A menu for the edit items
export class HeaderMenu extends React.PureComponent {  
  // Class constructor
  constructor(props) {
    // Collect props and set initial state
    super(props);
    this.state = {
      isFileVisible: false,
      isMenuVisible: false,
    }
  }

  // Render the edit menu
  render() {
    return (
      <div className="header">
        <div className="headerLeft">
          <div className="title">Minerva | EDIT MODE</div>
          <div class={"menuButton saveButton" + (this.props.saved ? " inactive" : "")} onClick={this.props.saveFile}>Save</div>
          <div className={"menuButton" + (this.state.isFileVisible ? " selected" : "")} onClick={() => this.setState((prevState) => { return { isFileVisible: !prevState.isFileVisible }})}>File
            {this.state.isFileVisible &&
              <div class="headerExpansion">
                <ConfirmButton buttonClass="expansionMenuButton" onClick={() => {switchPort(64636);}} buttonText="Normal Mode" />
                <div class="expansionMenuButton" onClick={newConfig}>New Config</div>
              </div>
            }
          </div>
        </div>
        <div className="headerRight">
          <ConfirmButton buttonClass="menuButton" onClick={() => {this.props.closeMinerva();}} buttonText="Quit Minerva" />
          <img src={logoWide} className="logo" alt="logo" />
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
      sceneList: [{id: -1, description: " "}],
      flag: true,
      isDeleteVisible: false,
      deleteId: -1,
    }

    // Bind functions
    this.loadScenes = this.loadScenes.bind(this);
    this.handleChange = this.handleChange.bind(this);
    this.newScene = this.newScene.bind(this);
  }

  // On render, pull the full scene list
  async componentDidMount() {
    this.loadScenes();
  }

  // Helper function to load available scenes
  async loadScenes() {
    try {
      // Fetch all scenes and process the response
      let response = await fetch(`/allScenes`);
      const json = await response.json();

      // If the response is valid
      if (json.isValid) {
        // Get the detail of each item
        let sceneList = [{id: -1, description: "Overall Configuration"}];
        await asyncForEach(json.data.items, async (item) => {
          // Fetch the description of the item
          response = await fetch(`/getItem/${item.id}`);
          const json2 = await response.json();

          // If valid, save the id and description
          if (json2.isValid) {
            sceneList.push({
              id: item.id,
              description: json2.data.item.description,
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
    this.props.changeScene(parseInt(e.target.value));

    // Save the delete id
    this.setState({ deleteId: parseInt(e.target.value) });
  }

  // Create a new scene
  async newScene() {
    // Get a list of all ids
    let response = await fetch(`allItems`);
    const json = await response.json();

    // If the response is valid
    if (json.isValid) {
      // Get the detail of each item
      let list = json.data.itemPairs;

      // Find the next unused ID
      let id = 1000;
      while (list.some((value) => value.id === id)) { id++ };
      
      // Compose the item into a modification
      let modifications = [{
        modifyItem: {
          itemPair: {
            id: id,
            description: "New Scene",
      }}}];

      // Add the scene definition to the modifications
      modifications.push({
        modifyScene: {
          itemId: { id: id },
          scene: {
            items: [],
          }
        }
      });

      // Save the new scene
      this.props.saveModifications(modifications);

      // After a moment, reload the scenes and change to that scene
      setTimeout(() => {
        this.loadScenes();
        this.props.changeScene(id);
      }, 100);
    } // FIXME fail silently
  }
  
  // Render the scene menu
  render() {
    // Compose the list of options
    let options = this.state.sceneList.map((scene) => <option key={scene.id.toString()} value={scene.id}>{scene.description}</option>);
    options.sort((first, second) => { return first.id - second.id } );

    // Return the complete menu
    return (
      <div className="sceneMenu">
        <div className="title">Select Scene:</div>
        <select className="select" value={this.props.value} onChange={this.handleChange}>
          {options}
        </select>
        {this.props.value !== -1 && <div className="deleteScene" onPointerDown={() => {this.setState({ isDeleteVisible: true })}}>
          Delete<br/>Scene
          {this.state.isDeleteVisible && <DeleteMenu id={this.state.deleteId} afterDelete={() => setTimeout(() => { this.loadScenes(); this.props.changeScene(-1); }, 100)} closeMenu={() => {this.setState({ isDeleteVisible: false })}} saveModifications={this.props.saveModifications} />}
        </div>}
        <div className="newScene" onPointerDown={this.newScene}>
          New<br/>Scene
        </div>
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
      ready: false,
    }

    // Create the search ref
    this.search = React.createRef();
 
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
      if (json.isValid) {
        // Save the temporary list, without types
        this.setState({
          unfiltered: json.data.itemPairs,
          ready: true,
        });

        // Get the detail of each item
        let list = [];
        await asyncForEach(json.data.itemPairs, async (item) => {
          // Check to see the item type
          let response = await fetch(`getType/${item.id}`);
          let type = "none";

          // If type is valid, save it
          const json = await response.json();
          if (json.isValid) {
            type = json.data.message;
          }

          // Save the id, type, and description
          list.push({
            id: item.id,
            type: type,
            description: item.description,
          });
        });

        // Save the result to the state
        this.setState({
          unfiltered: list,
          ready: true,
        });

        // Try to grab focus on the search input
        try {
          this.search.current.focus();
        } catch {
          // the window was closed before it finished loading
        }
      }
    
    // Ignore errors
    } catch (e) {
      console.log(`Server inaccessible.`);
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
    let list = this.state.filtered.map((item) => <div className={`divButton ${item.type}`} onClick={(e) => {this.props.addItem(item.id, this.props.left, this.props.top - 40)}}>{item.description}</div>)
    
    // Return the box
    return (
      <div className={`addMenu`} style={{ left: `${this.props.left}px`, top: `${this.props.top - 40}px` }} onClick={stopPropogation} onPointerDown={stopPropogation}>
        <div className="title">Add Item<span className="closeButton" onClick={this.props.closeMenu}>X</span></div>
        <input className="searchBar" ref={this.search} type="text" placeholder={this.state.ready ? "Type to search ..." : "  Loading ...  "} disabled={!this.state.ready} value={this.state.value} onInput={this.handleChange}></input>
        <div className="verticalScroll">
          <div>{list}</div>
        </div>
        {this.state.ready && <div className="addButton" onClick={() => {
          // Find the next unused ID
          let id = 1000;
          while (this.state.unfiltered.some((value) => value.id === id)) { id++ };
          
          // Compose the item into a modification
          let modifications = [{
            modifyItem: {
              itemPair: {
                id: id,
                description: "No Description",
          }}}];
          this.props.saveModifications(modifications);

          // Make the item visible
          this.props.addItem(id, this.props.left, this.props.top - 40)}}>+<div className="description">Add New</div></div>
        }
      </div>
    );
  }
}

// A select menu with a search bar
export class SelectMenu extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      value: null,
      unfiltered: [],
      filtered: [],
      ready: false,
    }

    // Create the search ref
    this.search = React.createRef();
 
    // Bind the various functions
    this.handleChange = this.handleChange.bind(this);
  }

  // On load, get the list of potential items
  async componentDidMount() {
    try {
      // Placeholder lists
      let list = [];
      let filtered = [];

      // Check if items were provided
      if (this.props.hasOwnProperty(`items`)) {
        // Check to see the item type for every item (not optimized)
        await asyncForEach(this.props.items, async (item) => {
          let response = await fetch(`getType/${item.id}`);
          let type = "none";
  
          // If type is valid, save it
          const json = await response.json();
          if (json.isValid) {
            type = json.data.message;
          }
          
          // If the menu type isn't none and this type doesn't match
          if (this.props.type !== "none" && this.props.type !== type) {
            return; // return early
          }
          
          // Fetch the description of the item
          response = await fetch(`getItem/${item.id}`);
          const json2 = await response.json();
  
          // If description is valid, save the id, type, and description
          if (json2.isValid) {
            list.push({
              id: item.id,
              type: type,
              description: json2.data.item.description,
            });
          }
        });

        // If a list was provided, show it immediately
        if (this.props.hasOwnProperty(`items`)) {
          filtered = [...list];
        }

        // Save the result to the state
        this.setState({
          unfiltered: list,
          filtered: filtered,
          ready: true,
        });
        
      // Otherwise, fetch all items and process the response
      } else {
        let response = await fetch(`allItems`);
        const json = await response.json();
        
        // If the response is valid
        if (json.isValid) {
          // Save the temporary list, without types
          this.setState({
            unfiltered: list,
            filtered: filtered,
            ready: true,
          });
          
          // Get the detail of each item
          await asyncForEach(json.data.itemPairs, async (item) => {
            // Check to see the item type
            let response = await fetch(`getType/${item.id}`);
            let type = "none";

            // If type is valid, save it
            const json = await response.json();
            if (json.isValid) {
              type = json.data.message;
            }
            
            // If the menu type isn't none and this type doesn't match
            if (this.props.type !== "none" && this.props.type !== type) {
              return; // return early
            }
            
            // Save the id, type, and description
            list.push({
              id: item.id,
              type: type,
              description: item.description,
            });
          });

          // Save the result to the state
          this.setState({
            unfiltered: list,
            filtered: filtered,
            ready: true,
          });
        }
      }

      // Try to grab focus on the search input
      try {
        this.search.current.focus();
      } catch {
        // the window was closed before it finished loading
      }
    
    // Ignore errors
    } catch {
      console.log(`Server inaccessible.`);
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
    let list = this.state.filtered.map((item) => <div className={`divButton ${item.type}`} onClick={() => {this.props.addItem(item.id)}}>{item.description}</div>)
    
    // Return the box
    return (
      <div className={`addMenu ${this.props.type}`} onClick={stopPropogation} onPointerDown={stopPropogation}>
        <div className="title">{`Select ${this.props.type.charAt(0).toUpperCase() + this.props.type.slice(1)}`}<span className="closeButton" onClick={this.props.closeMenu}>X</span></div>
        <input className="searchBar" ref={this.search} type="text" placeholder={this.state.ready ? "Type to search ..." : "  Loading ...  "} disabled={!this.state.ready} value={this.state.value} onInput={this.handleChange}></input>
        <div className="verticalScroll">
          <div>{list}</div>
        </div>
      </div>
    );
  }
}

// An add action menu
export class AddActionMenu extends React.PureComponent {
  // Return the completed box
  render() {
    // Compose the list of possible action types
    let actionList = [
      <div className="addActionButton" onClick={() => {this.props.addAction({ AdjustMedia: { adjustment: { channel: 0, direction: "Up" }}})}}>Adjust Media</div>,
      <div className="addActionButton" onClick={() => {this.props.addAction({ CancelEvent: { event: { id: 0 }}})}}>Cancel Event</div>,
      <div className="addActionButton" onClick={() => {this.props.addAction({ CueEvent: { event: { event_id: { id: 0 }}}})}}>Cue Event</div>,
      <div className="addActionButton" onClick={() => {this.props.addAction({ CueDmx: { fade: { channel: 1, value: 0 }}})}}>Cue Lights</div>,
      <div className="addActionButton" onClick={() => {this.props.addAction({ CueMedia: { cue: { uri: "", channel: 0 }}})}}>Cue Media</div>,
      <div className="addActionButton" onClick={() => {this.props.addAction({ ModifyStatus: { status_id: { id: 0 }, new_state: { id: 0 }}})}}>Modify Status</div>,
      <div className="addActionButton" onClick={() => {this.props.addAction({ NewScene: { new_scene: { id: 0 }}})}}>New Scene</div>,
      <div className="addActionButton" onClick={() => {this.props.addAction({ SelectEvent: { status_id: { id: 0 }, event_map: {}, }})}}>Select Event</div>,
      <div className="addActionButton" onClick={() => {this.props.addAction({ SaveData: { data: { StaticString: { string: "" }}}})}}>Save Data</div>,
      <div className="addActionButton" onClick={() => {this.props.addAction({ SendData: { data: { StaticString: { string: "" }}}})}}>Send Data</div>
    ];

    // Return the box
    return (
      <div className={`addActionMenu`} style={{ left: `${this.props.left}px`, top: `${this.props.top - 40}px` }} onClick={stopPropogation} onPointerDown={stopPropogation}>
        <div className="title">Add Action</div>
        <div className="verticalList">
          {actionList}
        </div>
      </div>
    );
  }
}

