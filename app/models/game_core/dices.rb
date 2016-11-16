class GameCore::Dices

  private

  def roll( amount = 2, type = 6 )
    1.upto( amount ).inject{ |s| s + rand( 1..type ) }
  end

end