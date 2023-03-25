import React from 'react';
import { Action } from './Actions';
import { ReceiveNode } from './Nodes';
import { State } from './States';
import { stopPropogation } from './Functions';
import { AddMenu, AddActionMenu, DeleteMenu } from './Menus';

// An item box to select the appropriate sub-box
export class ItemBox extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      left: 0, // horizontal offset of the area
      top: 0, // vertical offest of the area
      cursorX: 0, // starting point of the cursor
      cursorY: 0,
      description: "Loading ...", // placeholder for the real data
      display: {},
      type: "",
      isDeleteVisible: false,
    };

    // The timeout to save changes, if set
    this.saveTimeout = null;

    // Bind the various functions
    this.updateItem = this.updateItem.bind(this);
    this.handleMouseDown = this.handleMouseDown.bind(this);
    this.handleMouseMove = this.handleMouseMove.bind(this);
    this.handleMouseClose = this.handleMouseClose.bind(this);
    this.handleChange = this.handleChange.bind(this);
  }
  
  // Function to respond to clicking the area
  handleMouseDown(e) {
    stopPropogation();
   
    // Connect the mouse event handlers to the document
    document.onmousemove = this.handleMouseMove;
    document.onmouseup = this.handleMouseClose;

    // Get the current location
    let left = e.target.parentNode.offsetLeft;
    let top = e.target.parentNode.offsetTop;

    // Save the cursor position, hide the menu
    this.setState({
      left: left,
      top: top,
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
      let left = state.left - parseInt(changeX / this.props.zoom);
      let top = state.top - parseInt(changeY / this.props.zoom);
  
      // Enforce bounds on the new location
      left = (left >= 0) ? left : 0;
      top = (top >= 0) ? top : 0;
  
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

    // Save the updated location
    this.props.saveLocation(this.props.id, this.state.left, this.state.top);

    // Clear the left and top state
    this.setState({left: 0, top: 0});
  }

  // Helper function to update the item information
  async updateItem() {
    try {
      // Fetch the description of the status
      let response = await fetch(`getItem/${this.props.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.item.isValid) {
        // Save the itemPair
        this.setState({
          description: json.item.itemPair.description,
          display: json.item.itemPair.display,
        });
      }

      // Check to see the item type
      response = await fetch(`getType/${this.props.id}`);
      const json2 = await response.json();

      // If valid, save the result to the state
      if (json2.generic.isValid) {
        this.setState({
          type: json2.generic.message,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // On initial load, pull the location and item information
  componentDidMount() {
    // Pull the item information
    this.updateItem();
  }

  // Function to handle new text in the input
  handleChange(e) {
    // Extract the value
    let value = e.target.value;

    // Replace the existing description
    this.setState({
      description: value,
    });

    // CLear the existing timeout, if it exists
    if (this.saveTimeout) {
      clearTimeout(this.saveTimeout);
    }

    // Save the changes after a second pause
    let modifications = [{
      modifyItem: {
        itemPair: { id: this.props.id, description: value, display: this.state.display },
      },
    }];
    this.saveTimeout = setTimeout(() => {
      this.props.saveModifications(modifications);
    }, 1000);
  }
 
  // Return the selected box
  render() {
    // Calculate the left and top offsets
    let left = this.state.left ? `${this.state.left}px` : ``;
    let top = this.state.top ? `${this.state.top}px` : ``;

    // Return the item box
    return (
      <>
        {this.state.type !== "" &&
          <div id={`id-${this.props.id}`} className={`box ${this.state.type} row${this.props.row} ${this.props.isFocus ? 'focus' : ''}`} style={{ left: left, top: top }} onMouseDown={(e) => {stopPropogation(e); this.props.grabFocus(this.props.id)}}>
            <div className="title">
              <input type="text" value={this.state.description} size={this.state.description.length > 30 ? this.state.description.length - 10 : 20} onInput={this.handleChange}></input>
              <div className="disableSelect">({this.props.id})</div>
              {this.props.isFocus && <div className="removeMenu disableSelect">
                <div onMouseDown={(e) => {stopPropogation(e); this.props.removeItem(this.props.id)}}>Remove From Scene</div>
                <div onMouseDown={(e) => {stopPropogation(e); this.setState({ isDeleteVisible: true })}}>Delete</div>
              </div>}
              {this.state.isDeleteVisible && <DeleteMenu id={this.props.id} afterDelete={() => this.props.removeItem(this.props.id)} closeMenu={() => {this.setState({ isDeleteVisible: false })}} saveModifications={this.props.saveModifications} />}
            </div>
            <ReceiveNode id={`receive-node-${this.props.id}`} type={this.state.type} onMouseDown={(e) => {stopPropogation(e); this.handleMouseDown(e);}}/>
            {this.props.isFocus && this.state.type === "scene" && <SceneFragment id={this.props.id} changeScene={this.props.changeScene} saveModifications={this.props.saveModifications}/>}
            {this.props.isFocus && this.state.type === "status" && <StatusFragment id={this.props.id} grabFocus={this.props.grabFocus} createConnector={this.props.createConnector} saveModifications={this.props.saveModifications}/>}
            {this.props.isFocus && this.state.type === "event" && <EventFragment id={this.props.id} grabFocus={this.props.grabFocus} createConnector={this.props.createConnector} saveModifications={this.props.saveModifications}/>}
            {this.props.isFocus && (this.state.type === "label" || this.state.type === "none") && <BlankFragment id={this.props.id} updateItem={this.updateItem} saveModifications={this.props.saveModifications}/>}
          </div>
        }
      </>
    );
  }
}

// An empty box with no type
export class BlankFragment extends React.PureComponent {
  // Return the fragment
  render() {
    return (
      <>
        <div className="subtitle">Choose Item Type</div>
        <div className="typeChooser">
          <div className="divButton event" onClick={() => {let modifications = [{ modifyEvent: { itemId: { id: this.props.id }, event: [], }}]; this.props.saveModifications(modifications); this.props.updateItem()}}>Event</div>
          <div className="divButton status" onClick={() => {let modifications = [{ modifyStatus: { itemId: { id: this.props.id }, status: { MultiState: { current: { id: 0 }, allowed: [], no_change_silent: false, }}}}]; this.props.saveModifications(modifications); this.props.updateItem()}}>Status</div>
          <div className="divButton scene" onClick={() => {let modifications = [{ modifyScene: { itemId: { id: this.props.id }, scene: { events: [], }}}]; this.props.saveModifications(modifications); this.props.updateItem()}}>Scene</div>
        </div>
      </>
    );
  }
}

// A scene box with an scene and scene detail
export class SceneFragment extends React.PureComponent {
  // Return the fragment
  render() {
    return (
      <div className="divButton" onClick={() => {this.props.changeScene(this.props.id)}}>View This Scene</div>
    );
  }
}

// A statue box with a status and status detail
export class StatusFragment extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      status: {},
      isAddMenuVisible: "",
    }

    // Bind the various functions
    this.updateStatus = this.updateStatus.bind(this);
  }

  // Helper function to update the status information
  async updateStatus() {
    try {
      // Fetch the detail of the status
      const response = await fetch(`getStatus/${this.props.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.status.isValid) {
        this.setState({
          status: json.status.status,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // Helper function to add a new state
  addState(id) {
    if (this.state.status.hasOwnProperty(`MultiState`)) {
      // Save the new state id
      this.setState((prevState) => {
        // Compose the new status
        let newStatus = { MultiState:
          {...prevState.status.MultiState, allowed: [...prevState.status.MultiState.allowed, {id: id}]}
        };

        // Save the changes
        let modifications = [{
          modifyStatus: {
            itemId: { id: this.props.id },
            status: newStatus,
          },
        }];
        this.props.saveModifications(modifications);

        // Update the local state
        return {
          status: newStatus,
        };
      });
    
    // Ignore types other than multistate
    } else {
      console.error("This feature not yet implemented.");
    }
  }

  // Helper function to remove a state
  removeState(index) {
    if (this.state.status.hasOwnProperty(`MultiState`)) {
      // Save the new state id
      this.setState((prevState) => {
        // Remove the state from the allowed list
        let newAllowed = [...prevState.status.MultiState.allowed];
        newAllowed.splice(index, 1);

        // Compose the new status
        let newStatus = { MultiState:
          {...prevState.status.MultiState, allowed: newAllowed}
        };

        // Save the changes
        let modifications = [{
          modifyStatus: {
            itemId: { id: this.props.id },
            status: newStatus,
          },
        }];
        this.props.saveModifications(modifications);

        // Update the local state
        return {
          status: newStatus,
        };
      });
    
    // Ignore types other than multistate
    } else {
      console.error("This feature not yet implemented.");
    }
  }

  // On initial load, pull the status information
  componentDidMount() {
    // Pull the new status information
    this.updateStatus();
  }

  // Return the fragment
  render() {
    // Compose any states into a list
    let children = [];
    if (this.state.status.hasOwnProperty(`MultiState`)) {
      children = this.state.status.MultiState.allowed.map((state, index) => <State key={state.toString()} state={state} grabFocus={this.props.grabFocus} removeState={() => {this.removeState(index)}} createConnector={this.props.createConnector} />);
    }

    // Return the fragment
    return (
      <>
        <div className="subtitle">States:</div>
        <div className="verticalList" onWheel={stopPropogation}>{children}
          {this.state.selectMenu}
        </div>
        <div className="addButton" onClick={() => {this.setState(prevState => ({ isAddMenuVisible: !prevState.isAddMenuVisible }))}}>
          {this.state.isAddMenuVisible ? `-` : `+`}
          {this.state.isAddMenuVisible && <AddMenu type="event" left={180} top={60} addItem={(id) => {this.setState({ isAddMenuVisible: false }); this.addState(id)}} saveModifications={this.props.saveModifications}/>}
        </div>
      </>
    );
  }
}

// An event box with an event and event detail
export class EventFragment extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      eventActions: [], // placeholder for the read data
      isAddMenuVisible: false,
      selectMenu: null,
    }

    // Bind the various functions
    this.updateEvent = this.updateEvent.bind(this);
    this.addAction = this.addAction.bind(this);
    this.changeAction = this.changeAction.bind(this);
    this.selectMenu = this.selectMenu.bind(this);
  }

  // Helper function to update the event information
  async updateEvent() {
    try {
      // Fetch the detail of the event
      const response = await fetch(`getEvent/${this.props.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.event.isValid) {
        this.setState({
          eventActions: json.event.event.actions,
        });
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // Helper function to add a new event action
  addAction(action) {
    // Save the change action
    this.setState((prevState) => {
      // Copy the existing action list and add a blank action
      let newActions = [...prevState.eventActions, action];

      // Save the changes
      let modifications = [{
        modifyEvent: {
          itemId: { id: this.props.id },
          event: newActions,
        },
      }];
      this.props.saveModifications(modifications);

      // Update the local state
      return {
        eventActions: [...newActions],
        isAddMenuVisible: false,
      };
    });
  }

  // Helper function to change an event action
  changeAction(index, action) {
    // Save the change action
    this.setState((prevState) => {
      // Copy the existing action list
      let newActions = [...prevState.eventActions];

      // If an action was specified, replace it
      if (action) {
        newActions[index] = action;

      // Otherwise, remove that index number
      } else {
        newActions.splice(index, 1);
      }

      // Save the changes
      let modifications = [{
        modifyWebEvent: {
          itemId: { id: this.props.id },
          event: {
            actions: newActions,
          }
        },
      }];
      this.props.saveModifications(modifications);

      // Update the local state
      return {
        eventActions: [...newActions],
      };
    });
  }

  // Helper function to save the new select menu
  selectMenu(newMenu) {
    // Check to see if it's already set
    if (newMenu !== null && this.state.selectMenu !== null) {
      return false;
    }
    
    // Otherwise, update it and return true
    this.setState({
      selectMenu: newMenu,
    })
    return true;
  }

  // On initial load, pull the event information
  componentDidMount() {
    // Pull the new event information
    this.updateEvent();
  }

  // Return the fragment
  render() {
    // Compose any actions into a list
    const children = this.state.eventActions.map((action, index) => <Action key={action.toString()} action={action} grabFocus={this.props.grabFocus} changeAction={(newAction) => {this.changeAction(index, newAction)}} selectMenu={this.selectMenu} createConnector={this.props.createConnector}/>);

    // Return the fragment
    return (
      <>
        <div className="subtitle">Actions:</div>
        <div className="verticalList" onWheel={stopPropogation}>
          {children}
          {this.state.selectMenu}
        </div>
        <div className="addButton" onClick={() => {this.setState(prevState => ({ isAddMenuVisible: !prevState.isAddMenuVisible }))}}>
          {this.state.isAddMenuVisible ? `-` : `+`}
          {this.state.isAddMenuVisible && <AddActionMenu left={180} top={60} addAction={this.addAction}/>}
        </div>
      </>
    );
  }
}

