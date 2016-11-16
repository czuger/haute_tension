class GameCore::Dices

  def self.roll( amount = 2, type = 6 )
    1.upto( amount ).inject{ |s| s + rand( 1..type ) }
  end

end