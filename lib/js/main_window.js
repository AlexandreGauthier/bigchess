const { talkToMe } = require('../native');

const blank_board = "rnbqkbnrppppppppxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxPPPPPPPPRNBQKBNR";
const empty_piece = "res/pieces/x.png"
const piece_path_prefix = "res/pieces/"
const piece_path_suffix = ".svg"
square_size = 8;

function Square(piece, bg, index) {
    this.index = index;

    this.class = `h-20 w-20`
    if (bg != "") {
        this.class += " bg-"+bg
    }

    if (piece == "") {
        this.piece = empty_piece
    } else {
        this.piece = piece_path_prefix + piece + piece_path_suffix;
    }
}

function Blank_square(bg, index) {
    return new Square("", bg, index);
}

function board_from_string(string, bg_light, bg_dark) {
    var board = [];
    var bg_toggle = true
    for (var index =0; index < 64; index++) {
            var char = string[index];
            var bg = bg_toggle ? bg_light : bg_dark;
            bg_toggle = (index+1) % 8 != 0 ? !bg_toggle : bg_toggle;
            
            if (char == 'x') {
                board[index] = new Blank_square(bg, index);
                continue;
            }

           if (char.toUpperCase() == char) {
               var piece = "w"+char;
           } else {
               var piece = "b"+char.toUpperCase();
           }

           board[index] = new Square(piece, bg, index);
           }
    return board;
    }

Vue.component('squarec', {
    props: ["square"],
    template: `
        <div :class="square.class" v-on:click="clicked(square.index)">
             <img :src="square.piece">
        </div>
    `,
    methods: {
        clicked: function(index) {
            alert("This number (index*100) was calculated by the rust backend: "+ talkToMe(index))
        }
    }
})

var app = new Vue({
  el: '#app',
  data: {
    board: board_from_string(blank_board, "red-200", "red-400")
  }
})



// Disable dragging images
window.ondragstart = function() { return false; } 

