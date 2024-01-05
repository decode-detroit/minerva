import React from 'react';
import { ItemBox } from './Boxes';
import { AddMenu, SceneMenu, SelectMenu } from './Menus';
import { vh, vw, stopPropogation } from './Functions';
import { TextInput, ToggleSwitch } from './Buttons';

// A box to contain the draggable edit area
export class ViewArea extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props and set initial state
    super(props);
    this.state = {
      sceneId: -1, // the current scene id
      idList: [], // list of all item ids in this scene
      focusId: -1, // the highlighted item box
      connections: [], // list of all connectors
      top: 0, // edit area translation properties
      left: 0,
      zoom: 1, // edit area zoom setting
      cursorX: 0, // starting point of the cursor
      cursorY: 0,
      isMenuVisible: false, // flag to show the context menu
    }

    // Bind the various functions
    this.handlePointerDown = this.handlePointerDown.bind(this);
    this.handlePointerMove = this.handlePointerMove.bind(this);
    this.handlePointerClose = this.handlePointerClose.bind(this);
    this.handleWheel = this.handleWheel.bind(this);
    this.showContextMenu = this.showContextMenu.bind(this);
    this.changeScene = this.changeScene.bind(this);
    this.refresh = this.refresh.bind(this);
    this.addItemToScene = this.addItemToScene.bind(this);
    this.removeItemFromScene = this.removeItemFromScene.bind(this);
    this.createConnector = this.createConnector.bind(this);
    this.grabFocus = this.grabFocus.bind(this);
    this.saveLocation = this.saveLocation.bind(this);
    this.saveDimensions = this.saveDimensions.bind(this);
  }
  
  // Function to respond to clicking the area
  handlePointerDown(e) {
    stopPropogation(e);
   
    // Connect the pointer event handlers to the document
    document.onpointermove = this.handlePointerMove;
    document.onpointerup = this.handlePointerClose;

    // Save the cursor position, deselect any focus, and hide the menu
    this.setState({
      cursorX: e.clientX,
      cursorY: e.clientY,
      isMenuVisible: false,
      focusId: -1,
    });
  }

  // Function to respond to dragging the area
  handlePointerMove(e) {
    stopPropogation(e);

    // Update the state
    this.setState((state) => {
      // Calculate change from old cursor position
      let changeX = state.cursorX - e.clientX;
      let changeY = state.cursorY - e.clientY;
  
      // Calculate the new location
      let left = state.left - changeX;
      let top = state.top - changeY;
  
      // Enforce bounds on the new location
      left = (left <= 0) ? left : 0;
      top = (top <= 0) ? top : 0;
  
      // Save the new location and current cursor position
      return {
        left: left,
        top: top,
        cursorX: e.clientX,
        cursorY: e.clientY,
      }
    });
  }
  
  // Function to respond to releasing the pointer
  handlePointerClose() {
    // Stop moving when pointer button is released
    document.onpointermove = null;
    document.onpointerup = null;
  }

  // Function to respond to wheel events
  handleWheel(e) {

    return;
    // FIXME Disabled
    /*stopPropogation(e);

    // Extract the event, delta, and pointer location
    let delta = e.deltaY / 5000; // convert the speed of the zoom
    let locX = e.clientX; // FIXME triangulate correct location
    let locY = e.clientY;

    // Get the window dimensions
    let width = window.innerWidth;
    let height = window.innerHeight;

    // Update the zoom
    this.setState(prevState => {
      // Decrement the zoom
      let zoom = prevState.zoom - delta;

      // Check bounds
      if (zoom < 0.5) {
        zoom = 0.5;
      }
      if (zoom > 1) {
        zoom = 1;
      }

      // Update the top and left offset
      let top = -parseInt(((zoom / prevState.zoom) * (-prevState.top + (height/2))) - (height/2));
      let left = -parseInt(((zoom / prevState.zoom) * (-prevState.left + (width/2))) - (width/2));
      console.log(prevState.top, top);


      // Enforce bounds on the new location
      top = (top <= 0) ? top : 0;
      left = (left <= 0) ? left : 0;

      // Update
      return ({
        top: top,
        left: left,
        zoom: zoom,
      })
    });*/
  }

  // Function to show the context menu at the correct location
  showContextMenu(e) {
    stopPropogation(e);
    e.preventDefault(); // block the browser menu

    // Update the cusor location and mark the menu as visible
    this.setState({
      cursorX: e.clientX,
      cursorY: e.clientY,
      isMenuVisible: true,
    });

    return false;
  }

  // Function to change the displayed scene
  changeScene(id) {
    // Save the change
    this.setState({
      sceneId: id,
    });
  }

  // Function to hide the menu and add an item to the scene and the viewport
  addItemToScene(id, left, top) {
    // Add the item to the list, if missing
    this.setState((prevState) => {
      // If the id is not present
      if (!prevState.idList.includes(id)) {
        // Add it to the list
        const newList = [...prevState.idList, id];

        // Convert the item and group lists to item ids
        let items = newList.map((id) => {return { id: id }});

        // Submit the modification to the scene
        this.props.saveModifications([{ modifyScene: { itemId: { id: parseInt(this.state.sceneId) }, scene: { items: items, groups: [], }}}]); // FIXME copy key map

        // Save the location
        this.saveLocation(id, parseInt((left / this.state.zoom) - this.state.left), parseInt((top / this.state.zoom) - this.state.top));
        
        // Update the state
        return ({
          idList: newList,
          isMenuVisible: false,
        });
      }

      // Otherwise, just close the add menu
      return ({
        isMenuVisible: false,
      });
    });
  }

  // Function to hide remove an item from the scene and the viewport
  removeItemFromScene(id) {
    // Remove the item from the list, if present
    this.setState(prevState => {
      // Remove the id if present in the id list or group list
      if (prevState.idList.includes(id)) {
        // Filter from both lists
        const newIdList = prevState.idList.filter(listId => listId !== id);

        // Convert both lists to item ids
        let items = newIdList.map((id) => {return { id: id }});

        // Submit the modification to the scene
        this.props.saveModifications([{ modifyScene: { itemId: { id: parseInt(this.state.sceneId) }, scene: { items: items, groups: [], }}}]); // FIXME copy key map
        
        // Update the state
        return ({
          idList: newIdList,
        });
      }

      // Otherwise, do nothing
      return ({});
    });
  }

  // Function to grab focus for a specific item box
  grabFocus(id) {
    // Save the focus id
    this.setState({
      focusId: id,
    });
  }

  // A function to save the new location of an item to the stylesheet
  saveLocation(id, left, top) {
    // Save the new style rule
    this.props.saveStyle(`#scene-${this.state.sceneId} #id-${id}`, `{ left: ${left}px; top: ${top}px; }`);
  }
  
  // A function to save the new dimentions of a group area to the stylesheet
  saveDimensions(id, width, height) {
    // Save the new style rule
    this.props.saveStyle(`#scene-${this.state.sceneId} #id-${id} .groupArea`, `{ width: ${width}px; height: ${height}px; }`);
  }

  // Function to create a new connector between boxes
  createConnector(type, ref, id) {
    // Add the item, if it doesn't already exist
    //this.addItem(id); // FIXME handle this edge case
    
    // Add the connection
    this.setState({
      connections: [...this.state.connections, {type: type, ref: ref, id: id}],
    })
  }

  // Did update function to trigger refresh of idList
  async componentDidUpdate(prevProps, prevState) {
    // Update if the scene changed
    if (prevState.sceneId !== this.state.sceneId) {
      this.refresh();
    }
  }
    
  // Helper function to refresh all the displayed events
  async refresh() {
    // Check for the empty scene
    if (parseInt(this.state.sceneId) === -1) {
      // Save the empty list
      this.setState({
        idList: [], // reset the item list
        connections: [], // reset the connectors
      });
    
    // Otherwise, load the scene
    } else {
      try {
        // Fetch and convert the data
        const response = await fetch(`getScene/${this.state.sceneId}`)
        const json = await response.json();

        // If valid, save the result to the state
        if (json.isValid) {
          // Strip ids from the items
          let ids = json.data.scene.items.map((item) => item.id);
          ids.sort();

          // Update the state
          this.setState({
            idList: ids,
            connections: [], // reset the connectors
          });
        }

      // Ignore errors
      } catch {
        console.log("Server inaccessible.");
      }
    }
  }
  
  // Render the edit area inside the viewbox
  render() {
    // Add the scene to the list, if it doesn't already exist
    let idList = [...this.state.idList];
    if (!idList.includes(this.state.sceneId)) {
      idList = [this.state.sceneId, ...this.state.idList];
    }

    // Render the result
    return (
      <>
        <SceneMenu value={this.state.sceneId} changeScene={this.changeScene} saveModifications={this.props.saveModifications} />
        <div className="viewArea" onContextMenu={this.showContextMenu}>
          {this.state.sceneId === -1 && <ConfigArea filename={this.props.filename} handleFileChange={this.props.handleFileChange} saveModifications={this.props.saveModifications} openFile={this.props.openFile} /> }
          {this.state.sceneId !== -1 && <>
            <EditArea id={this.state.sceneId} idList={idList} focusId={this.state.focusId} top={this.state.top} left={this.state.left} zoom={this.state.zoom} handlePointerDown={this.handlePointerDown} handleWheel={this.handleWheel} connections={this.state.connections} grabFocus={this.grabFocus} createConnector={this.createConnector} changeScene={this.changeScene} removeItem={this.removeItemFromScene} saveModifications={this.props.saveModifications} saveLocation={this.saveLocation} saveDimensions={this.saveDimensions} />
            {this.state.isMenuVisible && <AddMenu left={this.state.cursorX} top={this.state.cursorY} closeMenu={() => this.setState({ isMenuVisible: false })} addItem={this.addItemToScene} saveModifications={this.props.saveModifications}/>}
          </>}
        </div>
      </>
    );
  }
}

// The configuration parameters area
export class ConfigArea extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props and set initial state
    super(props);

    // Set initial state
    this.state = {
      parameters: {
        identifier: { id: 0 },
        defaultScene: { id: 0 },
      },
      isMenuVisible: false,
      isFileChanged: false,
      defaultDescription: "Loading ...",
    }

    // Bind the various functions
    this.updateDefaultScene = this.updateDefaultScene.bind(this);
    this.updateIdentifier = this.updateIdentifier.bind(this);
    this.updateBackgroundProcess = this.updateBackgroundProcess.bind(this);
    this.updateParameters = this.updateParameters.bind(this);
    this.handleFileChange = this.handleFileChange.bind(this);
    this.toggleDefaultMenu = this.toggleDefaultMenu.bind(this);
  }

  // Helper function to update the default scene information
  async updateDefaultScene() {
    try {
      // Fetch the description of the status
      let response = await fetch(`getItem/${this.state.parameters.defaultScene.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.isValid) {
        this.setState({
          description: json.data.item.description,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // Function to handle new identifier value
  updateIdentifier(e) {
    // Extract the value
    let value = parseInt(e.target.value);

    // Update the parameters if it's valid
    if (!isNaN(value)) {
      this.updateParameters("identifier", { id: value });
    }
  }

  // Helper function to update the background process
  updateBackgroundProcess(key, value) {
    // Compose the new background process
    let new_background = {
      process: "",
      arguments: [],
      keepalive: false,
    };
    if (this.state.parameters.backgroundProcess) {
      new_background = {...this.state.parameters.backgroundProcess};
    }

    // Update the background process
    new_background[`${key}`] = value;

    // If the process is empty, use null instead
    if (new_background.process === "") {
      new_background = null;
    }
    
    // Save the change
    this.updateParameters("backgroundProcess", new_background);
  }

  // Helper function to update the parameters
  updateParameters(key, value) {
    // Save the parameter change
    this.setState((prevState) => {
      // Copy the current parameters
      let new_params = {...prevState.parameters};

      // Update the seleted value
      new_params[`${key}`] = value;

      // Save the changes
      let modifications = [{
        modifyParameters: {
          parameters: new_params,
        },
      }];
      this.props.saveModifications(modifications);

      // Update the local state
      return {
        parameters: new_params,
      };
    });
  }

  // Helper function to handle a change in the filename
  handleFileChange(e) {
    // Note the file change locally
    this.setState({isFileChanged: true}); 

    // Pass the file change up
    this.props.handleFileChange(e);
  }

  // Helper function to show or hide the default scene select menu
  toggleDefaultMenu() {
    // Set the new state of the menu
    this.setState(prevState => {
      return ({
        isMenuVisible: !prevState.isMenuVisible,
      });
    });
  }

  // On initial load, pull the description of the default scene
  async componentDidMount() {
    // Reload the configuration parameters
    const response = await fetch(`getConfigParam`);
    const json = await response.json();

    // If the response is valid
    if (json.isValid) {
      // Save the parameters
      this.setState({
        parameters: json.data.parameters,
      });
    }

    // Update the default scene listing
    this.updateDefaultScene();
  }

  // Did update function to trigger refresh of default scene
  async componentDidUpdate(prevProps, prevState) {
    // Update if the scene changed
    if (prevState.parameters.defaultScene.id !== this.state.parameters.defaultScene.id) {
      this.updateDefaultScene();
    }
  }

  // Render the configuration parameters area
  render() {
    return (
      <div id="configArea" className="configArea">
        <div>Filename:
          <TextInput value={this.props.filename} handleInput={this.handleFileChange} />
          <button class={"" + (this.state.isFileChanged ? "" : " disabled")} onClick={() => {this.setState({isFileChanged: false}); this.props.openFile()}}>Open File</button>
        </div>
        <div>Identifier:
          <input type="number" min="0" value={this.state.parameters.identifier.id} onInput={this.updateIdentifier} />
        </div>
        <div className="defaultScene">Default Scene:
          <span className={this.state.isMenuVisible && "isEditing"} onClick={this.toggleDefaultMenu}> {this.state.description}</span>
          {this.state.isMenuVisible && <SelectMenu type="scene" closeMenu={this.toggleDefaultMenu} addItem={(id) => {this.toggleDefaultMenu(); this.updateParameters("defaultScene", { id: id })}}/>}
        </div>
        <div>Backup Server Location:
          <TextInput value={this.state.parameters.serverLocation} handleInput={(e) => {this.updateParameters("serverLocation", `${e.target.value === "" ? null : e.target.value}`)}} />
        </div>
        <div>DMX Connection Path:
          <TextInput value={this.state.parameters.dmxPath} handleInput={(e) => {this.updateParameters("dmxPath", `${e.target.value === "" ? null : e.target.value}`)}} />
        </div>
        <div>Background Process:
          {!this.state.parameters.backgroundProcess && <TextInput value="" handleInput={(e) => {this.updateBackgroundProcess("process", `${e.target.value}`)}} />}
          {this.state.parameters.backgroundProcess && <>
            <TextInput value={this.state.parameters.backgroundProcess.process} handleInput={(e) => {this.updateBackgroundProcess("process", `${e.target.value}`)}} /><br/>
            Process Arguments: <TextInput value={this.state.parameters.backgroundProcess.arguments.join(' ')} handleInput={(e) => {this.updateBackgroundProcess("arguments", `${e.target.value}`.split(' '))}} /><br/>
            Keep Process Running? <ToggleSwitch value={this.state.parameters.backgroundProcess.keepalive} offOption="No" onOption="Yes" handleToggle={() => {this.updateBackgroundProcess("keepalive", !this.state.parameters.backgroundProcess.keepalive)}} />
          </>}
        </div>
        <div>System Connections: Not Yet Implemented</div>
        <div>Media Players: Not Yet Implemented</div>
      </div>
    )
  }s
}

// The draggable edit area
export class EditArea extends React.PureComponent {
  // Render the draggable edit area
  render() {
    // Create a box for each event
    const boxes = this.props.idList.map((id, index) => <ItemBox key={id.toString()} id={id} focusId={this.props.focusId} row={parseInt(index / 12)} zoom={this.props.zoom} grabFocus={this.props.grabFocus} changeScene={this.props.changeScene} saveModifications={this.props.saveModifications} saveLocation={this.props.saveLocation} saveDimensions={this.props.saveDimensions} removeItem={this.props.removeItem} removeText="Remove From Scene" createConnector={this.props.createConnector} />);
    
    // Render the event boxes
    return (
      <div id={`scene-${this.props.id}`} className="editArea" style={{ left: `${this.props.left}px`, top: `${this.props.top}px`, transform: `scale(${this.props.zoom})` }} onPointerDown={this.props.handlePointerDown} onWheel={this.props.handleWheel}>
        {boxes}
        <LineArea connections={this.props.connections}/>
      </div>
    )
  }
}

// The connector line area
export class LineArea extends React.PureComponent {
  // Render the connector line area
  render() {
    // Temporarily disable render
    return null;
    
    // For every connection, generate enpoints
    //this.props.connectors

    // Select the line color
    let lineColor = `#008106`;
    
    // Render the event boxes
    return (
      <svg width={vw(500)} height={vh(500) - 200}>
        <line x1="0" y1="0" x2={vw(500)} y2={vh(500) - 200} style={{ stroke: `${lineColor}`, strokeWidth: 5 }}/>
      </svg>
    )
  }
}
