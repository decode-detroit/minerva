import React from 'react';
import { ItemBox } from './Boxes';
import { AddMenu, SceneMenu } from './Menus';
import { vh, vw } from './Functions';

// A box to contain the draggable edit area
export class ViewArea extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props and set initial state
    super(props);
    this.state = {
      sceneId: -1, // the current scene id
      idList: [], // list of all shown item ids
      focusId: -1, // the highlighted item box
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
    this.removeItem = this.removeItem.bind(this);
    this.removeItemFromScene = this.removeItemFromScene.bind(this);
    this.createConnector = this.createConnector.bind(this);
    this.grabFocus = this.grabFocus.bind(this);
    this.saveLocation = this.saveLocation.bind(this);
  }
  
  // Function to respond to clicking the area
  handleMouseDown() {
    // Hide the menu
    this.setState({
      isMenuVisible: false,
      focusId: -1,
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

  // Function to add a specific item to the viewarea, if not present
  addItem(id) {
    // Add the item to the list, if missing
    this.setState(prevState => {
      // Add the id if not present
      if (!prevState.idList.includes(id)) {
        return ({
          idList: [...prevState.idList, id],
        });
      }
    });
  }

  // Function to hide the menu and add an item to the scene and the viewport
  addItemToScene(id, left, top) {
    // Add the item to the list, if missing
    this.setState(prevState => {
      // Add the id if not present
      if (!prevState.idList.includes(id)) {
        const newList = [...prevState.idList, id];

        // If not in the empty scene
        if (parseInt(this.state.sceneId) !== -1) {
          // Convert the list to item ids
          let events = newList.map((id) => {return { id: id }});

          // Submit the modification to the scene
          this.props.saveModifications([{ modifyScene: { itemId: { id: parseInt(this.state.sceneId) }, scene: { events: events, }}}]); // FIXME copy key map

          // Save the location
          this.saveLocation(id, left, top);
        }
        
        // Update the state
        return ({
          idList: newList,
          isMenuVisible: false,
        });
      }

      // Close the add menu
      return ({
        isMenuVisible: false,
      });
    });
  }

  // Function to remove a specific item to the viewarea, if present
  removeItem(id) {
    // Remove the item from the list, if present
    this.setState(prevState => {
      // Remove the id if present
      if (prevState.idList.includes(id)) {
        return ({
          idList: prevState.idList.filter(listId => listId !== id),
        });
      }
    });
  }

  // Function to hide remove an item from the scene and the viewport
  removeItemFromScene(id) {
    // Remove the item from the list, if present
    this.setState(prevState => {
      // Remove the id if present
      if (prevState.idList.includes(id)) {
        const newList = prevState.idList.filter(listId => listId !== id);

        // If not in the empty scene
        if (parseInt(this.state.sceneId) !== -1) {
          // Convert the list to item ids
          let events = newList.map((id) => {return { id: id }});

          // Submit the modification to the scene
          this.props.saveModifications([{ modifyScene: { itemId: { id: parseInt(this.state.sceneId) }, scene: { events: events, }}}]); // FIXME copy key map
        }
        
        return ({
          idList: newList,
          isMenuVisible: false,
        });
      }

      // Close the add menu
      return ({
        isMenuVisible: false,
      });
    });
  }

  // Function to grab focus for a specific item box
  grabFocus(id) {
    // Make sure the item exists
    this.addItem(id);
    
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
    // Update if the scene changed
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
        <SceneMenu value={this.state.sceneId} changeScene={this.changeScene} saveModifications={this.props.saveModifications} />
        <div className="viewArea" onContextMenu={this.showContextMenu} onMouseDown={this.handleMouseDown}>
          {this.state.sceneId !== -1 && <>
            <EditArea id={this.state.sceneId} idList={this.state.idList} focusId={this.state.focusId} connections={this.state.connections} grabFocus={this.grabFocus} createConnector={this.createConnector} changeScene={this.changeScene} removeItem={this.removeItemFromScene} saveModifications={this.props.saveModifications} saveLocation={this.saveLocation} />
            {this.state.isMenuVisible && <AddMenu left={this.state.cursorX} top={this.state.cursorY} addItem={this.addItemToScene} saveModifications={this.props.saveModifications}/>}
          </>}
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
      zoom: 1, // zoom scaling for the edit window
      cursorX: 0, // starting point of the cursor
      cursorY: 0,
    }

    // Bind the various functions
    this.handleMouseDown = this.handleMouseDown.bind(this);
    this.handleMouseMove = this.handleMouseMove.bind(this);
    this.handleMouseClose = this.handleMouseClose.bind(this);
    this.handleWheel = this.handleWheel.bind(this);
  }

  // Function to respond to clicking the area
  handleMouseDown(e) {
    // Prevent any other event handlers
    e = e || window.event;
    e.preventDefault();
   
    // Connect the mouse event handlers to the document
    document.onmousemove = this.handleMouseMove;
    document.onmouseup = this.handleMouseClose;

    // Save the cursor position, deselect any focus
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

  // Function to respond to wheel events
  handleWheel(event) {
    // Zoom out
    if (event.deltaY > 0) {
      this.setState(prevState => {
        // Decrement the zoom
        let newZoom = prevState.zoom - (event.deltaY / 5000);
        
        // Check bounds
        if (newZoom < 0.5) {
          newZoom = 0.5;
        }

        // Update
        return ({
          zoom: newZoom,
        })
      });
    
    // Zoom in
    } else {
      this.setState(prevState => {
        // Decrement the zoom
        let newZoom = prevState.zoom - (event.deltaY / 5000);
        
        // Check bounds
        if (newZoom > 1) {
          newZoom = 1;
        }

        // Update
        return ({
          zoom: newZoom,
        })
      });
    }
  }

  // Render the draggable edit area
  render() {
    // Create a box for each event
    const boxes = this.props.idList.map((id, index) => <ItemBox key={id.toString()} isFocus={this.props.focusId === id} id={id} row={parseInt(index / 12)} grabFocus={this.props.grabFocus} changeScene={this.props.changeScene} saveModifications={this.props.saveModifications} saveLocation={this.props.saveLocation} removeItem={this.props.removeItem} createConnector={this.props.createConnector} />);
    
    // Render the event boxes
    return (
      <div id={`scene-${this.props.id}`} className="editArea" style={{ left: `calc(${this.state.left}px - (250% * ${1 - this.state.zoom}))`, top: `calc(${this.state.top}px - (250% * ${1 - this.state.zoom}))`, transform: `scale(${this.state.zoom})` }} onMouseDown={this.handleMouseDown} onWheel={this.handleWheel}>
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
