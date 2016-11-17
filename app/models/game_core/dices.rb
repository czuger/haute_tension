class GameCore::Dices

  def self.roll( amount = 2, type = 6 )
    1.upto( amount ).inject{ |s| s + rand( 1..type ) }
  end

  def self.roll_and_return_separated_result( amount = 2, type = 6 )
    1.upto( amount ).map{ |_| rand( 1..type ) }
  end

end