import React from 'react';
import { stopPropogation } from './Functions';

// An item box to cue the selected item
export class ItemBox extends React.PureComponent {
  // Class constructor
  constructor(props) {
    // Collect props
    super(props);

    // Set initial state
    this.state = {
      description: "Loading ...", // placeholder for the real data
      type: "",
      showGroup: false,
      groupIds: [],
    };

    // Bind the various functions
    this.updateItem = this.updateItem.bind(this);
    this.handleMouseDown = this.handleMouseDown.bind(this);
  }
  
  // Function to respond to clicking the area
  handleMouseDown(e) {
    stopPropogation(e);

    // If the item is an event
    if (this.state.type === "event") {
      // Trigger the selected event
      let cueEvent = {
        id: this.props.id,
        secs: 0,
        nanos: 0,
      };
      fetch(`/cueEvent`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify(cueEvent),
      }); // FIXME ignore errors
    
    // Otherwise, if this item is a group
    } else if (this.state.type === "group") {
      // Toggle the hidden items
      this.setState((prevState) => {
        // Update the local state
        return {
          showGroup: !prevState.showGroup,
        };
      });
    }
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

      // Check to see the item type
      response = await fetch(`getType/${this.props.id}`);
      const json2 = await response.json();

      // If valid, save the result to the state
      if (json2.isValid) {
        this.setState({
          type: json2.data.message,
        });

        // If it's a group, update the group
        if (json2.data.message === "group") {
          // Pull the new group information
          this.updateGroup();
        }
      }
    
    // Ignore errors
    } catch {
      console.log("Server inaccessible.");
    }
  }

  // Helper function to update the group information
  async updateGroup() {
    try {
      // Fetch the detail of the status
      const response = await fetch(`getGroup/${this.props.id}`);
      const json = await response.json();

      // If valid, save the result to the state
      if (json.isValid) {
        // Strip ids from the items
        let ids = json.data.group.items.map((item) => item.id);

        // Exclude this id to prevent recursion
        let clean_ids = ids.filter((id) => id !== this.props.id)

        // Sort the ids and save        
        clean_ids.sort();
        this.setState({
          showGroup: !json.data.group.isHidden,
          groupIds: clean_ids,
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
 
  // Return the selected box
  render() {
    // Return the item box
    return (
      <>
        {this.state.type !== "" &&
          <div id={`id-${this.props.id}`} className={`box ${this.state.type} row${this.props.row}`}>
            <div className="title" onMouseDown={this.handleMouseDown}>
              <div>{this.state.description}</div>
              <div className="subtitle">{this.props.id}</div>
              {this.state.type ==="group" && <div className="note">Click to {this.state.showGroup ? `hide` : `show`}</div>}
            </div>
            {this.state.type === "group" && this.state.showGroup && <GroupFragment idList={this.state.groupIds} />}
          </div>
        }
      </>
    );
  }
}

// A group box with other boxes inside
export class GroupFragment extends React.PureComponent {
  // Return the fragment
  render() {
    // Create a box for items that are in this group
    const boxes = this.props.idList.map((id, index) => <ItemBox key={id.toString()} id={id} row={parseInt(index / 12)} />);
    
    // Return the group box
    return (
      <div className="groupArea">
        {boxes}
      </div>
    );
  }
}
