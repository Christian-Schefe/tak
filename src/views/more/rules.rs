use dioxus::prelude::*;

#[component]
pub fn Rules() -> Element {
    rsx! {
        div { id: "rules-view",
            h1 { "How to play Tak" }
            p {
                "Tak is played on a square board with even spaces like a chessboard. Unlike most other strategy games, however, it can be played on several different-sized boards. At the beginning of the game, the board starts empty and each player is given the appropriate number of stones for the board size, as listed in the table below."
            }
            table {
                thead {
                    tr {
                        th { "Board Size" }
                        th { "Stones" }
                        th { "Capstones" }
                    }
                }
                tbody {
                    tr {
                        td { "4×4" }
                        td { "15" }
                        td { "0" }
                    }
                    tr {
                        td { "5×5" }
                        td { "21" }
                        td { "1" }
                    }
                    tr {
                        td { "6×6" }
                        td { "30" }
                        td { "1" }
                    }
                    tr {
                        td { "7×7" }
                        td { "40" }
                        td { "2" }
                    }
                    tr {
                        td { "8×8" }
                        td { "50" }
                        td { "2" }
                    }
                }
            }
            h2 { "Starting Play" }
            p {
                "Players alternate turns throughout the game. You must play on your turn - there is no option to pass. Tak is played with only orthogonal movement and connection; squares are not connected diagonally and diagonal movement is not possible. On each player's first turn, they will place one of their opponent's stones flat on any empty square of the board. Play then continues with players placing new stones or moving existing stones they control."
            }
            h2 { "On Your Turn" }
            p {
                "On each turn, you can do one of two things: place a stone on an empty space, or move stones you control."
            }
            h3 { "Placing Stones" }
            p {
                "On your turn, you can opt to place a stone from your reserve onto any empty square on the board. There are three stone types that can be placed: Flat Stone - The basic stone, laid flat on its face. This is what you use to build your road. Standing Stone - The basic stone, but standing on an edge. Also called a wall. This does not count as part of a road, but other stones cannot stack on top of it. Capstone - This is the most powerful piece. It, like a flat stone, counts as part of your road. Other stones cannot stack on top of it. The capstone also has the ability to move by itself onto a standing stone and flatten the standing stone into a flat stone. You can flatten both your opponent's and your own standing stones in this way."
            }
            h3 { "Moving Stones" }
            p {
                "The other option on your turn is to move stones that you control. If your stone is on the top of a stack, you control that entire stack. All three stone types (flat, standing, and cap) can be moved, and moving is the only way to create stacks. There is no limit to how tall a stack can be. When moving stacks of stones, you cannot move more stones than the size of the edge of the board; this is called the carry limit. On a 5×5 board, this means you cannot pick up more than 5 stones from a stack. On a 6×6 board the carry limit is 6, and so on."
            }
            h3 { "There are several simple steps to executing a stack move:" }
            p {
                "Pick up any number of stones up to the carry limit for the board you're playing on. Do not change the order of these stones. Move in a straight line in the direction of your choice - no diagonals and no changing direction. You must drop at least one stone from the bottom of the stack in your hand on each square you move over. You do not need to leave a stone in that stack’s starting space. You may not jump over walls or capstones. The capstone, if on the stack, may drop by itself onto a standing stone at the end of a move to flatten it."
            }
            h2 { "Winning" }
            p {
                "The object of Tak is to connect any two opposite edges of the board with your flat stones and capstone, creating a road. Any square or stack you control can count as part of a road(except ones with walls on them), but stones in a stack controlled by the other player do not. A road does not have to be a straight line; it can zig-zag across the board as long as all squares in the road are adjacent, not diagonal. If a player makes a single move that creates a road for both players, then the player who made the move wins. In the event that neither player creates a road and the board is either completely filled (no empty squares) or one of the players places their last piece, a secondary win condition comes into effect. When either of those cases is met, the game immediately ends and the winner is determined by counting who has more flat stones controlling the board. Only flat stones on the top of stacks or solely occupying a square are counted. The player with the higher flat count wins. A tie in the count results in a tie game."
            }
        }
    }
}
