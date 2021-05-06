import React from 'react';
import { ItemBox } from './Boxes';
import { AddMenu, SceneMenu } from './Menus';
import { vh, vw } from './functions';

// A box to contain the draggable edit area
export class ViewArea extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props and set initial state
    super(props);
    this.state = {
      sceneId: -1, // the current scene id
      idList: [], // list of all shown item ids
      focusId: 0, // the highlighted item box
      connections: [], // list of all connectors
      cursorX: 0, // starting point of the cursor
      cursorY: 0,
      isMenuVisible: false, // flag to show the context menu
    }

    // Bind the various functions
    this.handleMouseDown = this.handleMouseDown.bind(this);
    this.showContextMenu = this.showContextMenu.bind(this);
    this.changeScene = this.changeScene.bind(this);
    this.refresh = this.refresh.bind(this);
    this.addItem = this.addItem.bind(this);
    this.addItemToScene = this.addItemToScene.bind(this);
    this.createConnector = this.createConnector.bind(this);
    this.grabFocus = this.grabFocus.bind(this);
  }
  
  // Function to respond to clicking the area
  handleMouseDown() {
    // Hide the menu
    this.setState({
      isMenuVisible: false,
    });
  }

  // Function to show the context menu at the correct location
  showContextMenu(e) {
    // Prevent the default event handler
    e = e || window.event;
    e.preventDefault();

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

  // Function to hide the menu and add an item to the scene and the viewport FIXME also add to scene
  addItemToScene(id) {
    // FIXME add the item to current scene

    // Add the item to the viewarea
    this.addItem(id);

    // Close the add menu
    this.setState({
      isMenuVisible: false,
    });
  }

  // Function to add a specific item to the viewarea, if not present
  addItem(id) {
    // Add the item to the list, if missing
    if (!this.state.idList.includes(id)) {
      this.setState({
        idList: [...this.state.idList, id],
      });
    } 
  }

  // Function to grab focus for a specific item box
  grabFocus(id) {
    // Make sure the item exists
    this.addItem(id);
    
    // Save the focus id
    this.setState({
      focusId: id,
    })
  }

  // Function to create a new connector between boxes
  createConnector(type, ref, id) {
    // Add the item, if it doesn't already exist
    this.addItem(id);
    
    // Add the connection
    this.setState({
      connections: [...this.state.connections, {type: type, ref: ref, id: id}],
    })
  }

  // Did update function to trigger refresh of idList
  async componentDidUpdate(prevProps, prevState) {
    if (prevState.sceneId !== this.state.sceneId) {
      this.refresh();
    }
  }
    
  // Helper function to refresh all the displayed events
  async refresh() {
    // Check for the empty scene
    if (parseInt(this.state.sceneId) === -1) {
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
        if (json.scene.isValid) {
          // Strip ids from the items
          let ids = json.scene.scene.events.map((item) => item.id);
          ids.sort();

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
    return (
      <>
        <SceneMenu value={this.state.sceneId} changeScene={this.changeScene}/>
        <div className="viewArea" onContextMenu={this.showContextMenu} onMouseDown={this.handleMouseDown}>
          <EditArea idList={this.state.idList} focusId={this.state.focusId} connections={this.state.connections} grabFocus={this.grabFocus} createConnector={this.createConnector} changeScene={this.changeScene}/>
          {this.state.isMenuVisible && <AddMenu left={this.state.cursorX} top={this.state.cursorY} addItem={this.addItemToScene}/>}
        </div>
      </>
    );
  }
}

// The draggable edit area
export class EditArea extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props and set initial state
    super(props);
    this.state = {
      left: 0, // horizontal offset of the area
      top: 0, // vertical offest of the area
      cursorX: 0, // starting point of the cursor
      cursorY: 0,
    }

    // Bind the various functions
    this.handleMouseDown = this.handleMouseDown.bind(this);
    this.handleMouseMove = this.handleMouseMove.bind(this);
    this.handleMouseClose = this.handleMouseClose.bind(this);
  }

  // Function to respond to clicking the area
  handleMouseDown(e) {
    // Prevent any other event handlers
    e = e || window.event;
    e.preventDefault();
   
    // Connect the mouse event handlers to the document
    document.onmousemove = this.handleMouseMove;
    document.onmouseup = this.handleMouseClose;

    // Save the cursor position, hide the menu
    this.setState({
      cursorX: e.clientX,
      cursorY: e.clientY,
    });
  }

  // Function to respond to dragging the area
  handleMouseMove(e) {
    // Prevent the default event handler
    e = e || window.event;
    e.preventDefault();

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
  
  // Function to respond to releasing the mouse
  handleMouseClose() {
    // Stop moving when mouse button is released
    document.onmousemove = null;
    document.onmouseup = null;
  }

  // Render the draggable edit area
  render() {
    // Create a box for each event
    let offset = 0;
    const boxes = this.props.idList.map((id) => <ItemBox key={id.toString()} isFocus={this.props.focusId === id} left={100 + 275 * parseInt(offset / 6)} top={100 + 100 * ((offset++) % 6)} id={id} grabFocus={this.props.grabFocus} createConnector={this.props.createConnector} changeScene={this.props.changeScene}/>);
    
    // Render the event boxes
    return (
      <div className="editArea" style={{ left: `${this.state.left}px`, top: `${this.state.top}px` }} onMouseDown={this.handleMouseDown}>
        <LineArea connections={this.props.connections}/>
        {boxes}
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
