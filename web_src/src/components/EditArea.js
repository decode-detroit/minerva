import React from 'react';
import { EventBox } from './Boxes';
import { AddMenu, SceneMenu } from './Menus';

// A box to contain the draggable edit area
export class ViewArea extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props and set initial state
    super(props);
    this.state = {
      sceneId: -1, // the current scene id
      itemList: [], // list of all shown items
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
    this.createConnector = this.createConnector.bind(this);
  }
  
  // Function to respond to clicking the area
  handleMouseDown(e) {
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
    })
  }

  // Function to add a specific item to the item list, if not present
  addItem(id) {
    // Add the item to the list, if missing
    if (!this.state.itemList.includes(id)) {
      this.setState({
        itemList: [...this.state.itemList, {id: id}],
      });
    } 
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

  // Did update function to trigger refresh of itemList
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
        itemList: [], // reset the item list
        connections: [], // reset the connectors
      });
    
    // Otherwise, load the scene
    } else {
      try {
        // Fetch and convert the data
        const response = await fetch(`/getScene/${this.state.sceneId}`)
        const json = await response.json();

        // If valid, save the result to the state
        if (json.scene.isValid) {
          this.setState({
            itemList: json.scene.scene.events,
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
        <SceneMenu changeScene={this.changeScene}></SceneMenu>
        <div className="viewArea" onContextMenu={this.showContextMenu} onMouseDown={this.handleMouseDown}>
          <EditArea itemList={this.state.itemList} connections={this.state.connections} createConnector={this.createConnector}></EditArea>
          {this.state.isMenuVisible && <AddMenu left={this.state.cursorX} top={this.state.cursorY - 30 - vmin(1)}></AddMenu>}
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
    const boxes = this.props.itemList.map((item) => <EventBox key={item.id.toString()} left={100} top={100} id={item.id} createConnector={this.props.createConnector}></EventBox>);
    
    // Render the event boxes
    return (
      <div className="editArea" style={{ left: `${this.state.left}px`, top: `${this.state.top}px` }} onMouseDown={this.handleMouseDown}>
        <LineArea connections={this.props.connections}></LineArea>
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

// Helper functions for calculating box offset
function vh(v) {
  var h = Math.max(document.documentElement.clientHeight, window.innerHeight || 0);
  return (v * h) / 100;
}

function vw(v) {
  var w = Math.max(document.documentElement.clientWidth, window.innerWidth || 0);
  return (v * w) / 100;
}

function vmin(v) {
  return Math.min(vh(v), vw(v));
}
